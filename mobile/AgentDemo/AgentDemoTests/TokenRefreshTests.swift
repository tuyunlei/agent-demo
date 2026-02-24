import GRPCClient
import GRPCCore
import GRPCNIOTransportHTTP2
import Synchronization
import Testing

struct TokenRefreshTests {
    /// Token refresh succeeds → original request retried successfully.
    @Test func refreshSuccessThenRetry() async throws {
        // Chat service fails first call with UNAUTHENTICATED, succeeds after
        let mockChat = MockChatServiceImpl(
            sessionID: "session-ok",
            assistantText: "Retried OK",
            failUntilCall: 1
        )
        let mockAuth = MockAuthServiceImpl()
        let server = MockGRPCServer(services: [mockAuth, mockChat])
        try await server.start()
        defer { server.stop() }

        let baseClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true
        )
        let tokenStore = TokenStore(
            accessToken: "expired-token",
            refreshToken: "valid-refresh",
            refreshHandler: { refreshToken in
                let authClient = AuthServiceClient(apiClient: baseClient)
                let response = try await authClient.refreshToken(token: refreshToken)
                return (response.tokenPair.accessToken, response.tokenPair.refreshToken)
            }
        )
        let authedClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true,
            tokenStore: tokenStore
        )
        let chatClient = ChatServiceClient(apiClient: authedClient)

        let response = try await chatClient.sendMessage(
            requestID: "req-1",
            text: "Hello",
            sessionID: "",
            agentID: ""
        )

        #expect(response.assistantContent[0].text.text == "Retried OK")
        #expect(mockAuth.refreshCallCount == 1)
        #expect(mockChat.callCount == 2) // first failed, second succeeded

        // TokenStore should have the refreshed tokens
        let newAccess = await tokenStore.accessToken
        #expect(newAccess == "refreshed-access-token")
    }

    /// Refresh also fails → onAuthExpired triggered, error propagated.
    @Test func refreshFailureTriggersAuthExpired() async throws {
        // Chat service always returns UNAUTHENTICATED
        let mockChat = MockChatServiceImpl(failUntilCall: .max)
        let mockAuth = MockAuthServiceImpl(refreshShouldFail: true)
        let server = MockGRPCServer(services: [mockAuth, mockChat])
        try await server.start()
        defer { server.stop() }

        let baseClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true
        )

        let authExpiredCalled = Mutex<Bool>(false)
        let tokenStore = TokenStore(
            accessToken: "expired-token",
            refreshToken: "also-expired",
            refreshHandler: { refreshToken in
                let authClient = AuthServiceClient(apiClient: baseClient)
                let response = try await authClient.refreshToken(token: refreshToken)
                return (response.tokenPair.accessToken, response.tokenPair.refreshToken)
            }
        )
        tokenStore.onAuthExpired = {
            authExpiredCalled.withLock { $0 = true }
        }

        let authedClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true,
            tokenStore: tokenStore
        )
        let chatClient = ChatServiceClient(apiClient: authedClient)

        await #expect(throws: (any Error).self) {
            _ = try await chatClient.sendMessage(
                requestID: "req-1",
                text: "Hello",
                sessionID: "",
                agentID: ""
            )
        }

        #expect(authExpiredCalled.withLock { $0 })
        #expect(mockAuth.refreshCallCount == 1)
    }

    /// Two concurrent UNAUTHENTICATED responses share a single refresh call.
    @Test func concurrentRefreshOnlyCallsOnce() async throws {
        // Chat service fails first 2 calls, succeeds after
        let mockChat = MockChatServiceImpl(
            sessionID: "session-ok",
            assistantText: "OK",
            failUntilCall: 2
        )
        let mockAuth = MockAuthServiceImpl()
        let server = MockGRPCServer(services: [mockAuth, mockChat])
        try await server.start()
        defer { server.stop() }

        let baseClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true
        )
        let tokenStore = TokenStore(
            accessToken: "expired-token",
            refreshToken: "valid-refresh",
            refreshHandler: { refreshToken in
                let authClient = AuthServiceClient(apiClient: baseClient)
                let response = try await authClient.refreshToken(token: refreshToken)
                return (response.tokenPair.accessToken, response.tokenPair.refreshToken)
            }
        )
        let authedClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true,
            tokenStore: tokenStore
        )
        let chatClient = ChatServiceClient(apiClient: authedClient)

        // Launch two requests concurrently
        async let result1: Ai_Agent_Platform_V1_SendMessageResponse = chatClient.sendMessage(
            requestID: "req-1",
            text: "Hello 1",
            sessionID: "",
            agentID: ""
        )
        async let result2: Ai_Agent_Platform_V1_SendMessageResponse = chatClient.sendMessage(
            requestID: "req-2",
            text: "Hello 2",
            sessionID: "",
            agentID: ""
        )

        let (response1, response2) = try await(result1, result2)
        #expect(response1.assistantContent[0].text.text == "OK")
        #expect(response2.assistantContent[0].text.text == "OK")

        // RefreshToken RPC should have been called exactly once
        #expect(mockAuth.refreshCallCount == 1)
    }
}
