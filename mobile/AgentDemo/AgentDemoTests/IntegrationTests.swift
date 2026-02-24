import GRPCClient
import GRPCCore
import GRPCNIOTransportHTTP2
import Testing

struct IntegrationTests {
    @Test func loginViaMockServer() async throws {
        let mockAuth = MockAuthServiceImpl(
            accessToken: "test-access-token",
            refreshToken: "test-refresh-token",
            userID: "test-user-id"
        )
        let server = MockGRPCServer(services: [mockAuth])
        try await server.start()
        defer { server.stop() }

        let apiClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true
        )
        let authClient = AuthServiceClient(apiClient: apiClient)

        let response = try await authClient.login(
            email: "test@example.com",
            password: "password123"
        )

        #expect(response.userID == "test-user-id")
        #expect(response.tokenPair.accessToken == "test-access-token")
        #expect(response.tokenPair.refreshToken == "test-refresh-token")
    }

    @Test func sendMessageViaMockServer() async throws {
        let mockChat = MockChatServiceImpl(
            sessionID: "session-abc",
            assistantText: "Mock reply"
        )
        let server = MockGRPCServer(services: [mockChat])
        try await server.start()
        defer { server.stop() }

        let tokenStore = TokenStore(
            accessToken: "mock-token",
            refreshHandler: { _ in throw TokenRefreshError.noRefreshToken }
        )
        let apiClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true,
            tokenStore: tokenStore
        )
        let chatClient = ChatServiceClient(apiClient: apiClient)

        let response = try await chatClient.sendMessage(
            requestID: "req-1",
            text: "Hello",
            sessionID: "",
            agentID: ""
        )

        #expect(response.sessionID == "session-abc")
        #expect(response.assistantContent.count == 1)
        #expect(response.assistantContent[0].text.text == "Mock reply")
    }

    @Test func fullFlowLoginThenChatThenHistory() async throws {
        let mockAuth = MockAuthServiceImpl()
        let mockChat = MockChatServiceImpl(sessionID: "session-full", assistantText: "Hi there!")
        let mockSession = MockSessionServiceImpl(messages: [
            makeChatMessage(role: "user", text: "Hello"),
            makeChatMessage(role: "assistant", text: "Hi there!"),
        ])

        let server = MockGRPCServer(services: [mockAuth, mockChat, mockSession])
        try await server.start()
        defer { server.stop() }

        let baseClient = try APIClient(host: "127.0.0.1", port: server.port, usePlaintext: true)

        // 1. Login
        let authClient = AuthServiceClient(apiClient: baseClient)
        let loginResponse = try await authClient.login(email: "test@example.com", password: "pass")
        #expect(!loginResponse.tokenPair.accessToken.isEmpty)

        // 2. Create authenticated client with tokens
        let authedClient = try makeAuthenticatedClient(
            baseClient: baseClient, tokenPair: loginResponse.tokenPair
        )

        // 3. Send message
        let chatClient = ChatServiceClient(apiClient: authedClient)
        let chatResponse = try await chatClient.sendMessage(
            requestID: "req-1", text: "Hello", sessionID: "", agentID: ""
        )
        #expect(chatResponse.sessionID == "session-full")

        // 4. Load history
        let sessionClient = SessionServiceClient(apiClient: authedClient)
        let historyResponse = try await sessionClient.listMessages(
            sessionID: chatResponse.sessionID
        )
        #expect(historyResponse.messages.count == 2)
        #expect(historyResponse.messages[0].role == "user")
        #expect(historyResponse.messages[1].role == "assistant")
    }

    @Test func loginFailureReturnsError() async throws {
        let mockAuth = MockAuthServiceImpl(loginShouldFail: true)
        let server = MockGRPCServer(services: [mockAuth])
        try await server.start()
        defer { server.stop() }

        let apiClient = try APIClient(
            host: "127.0.0.1",
            port: server.port,
            usePlaintext: true
        )
        let authClient = AuthServiceClient(apiClient: apiClient)

        await #expect(throws: (any Error).self) {
            _ = try await authClient.login(
                email: "wrong@example.com",
                password: "bad-password"
            )
        }
    }

    private func makeAuthenticatedClient(
        baseClient: APIClient,
        tokenPair: Ai_Agent_Platform_V1_TokenPair
    ) throws -> APIClient {
        let tokenStore = TokenStore(
            accessToken: tokenPair.accessToken,
            refreshToken: tokenPair.refreshToken,
            refreshHandler: { refreshToken in
                let authClient = AuthServiceClient(apiClient: baseClient)
                let response = try await authClient.refreshToken(token: refreshToken)
                return (response.tokenPair.accessToken, response.tokenPair.refreshToken)
            }
        )
        return APIClient(
            host: baseClient.host,
            port: baseClient.port,
            usePlaintext: baseClient.usePlaintext,
            tokenStore: tokenStore
        )
    }

    private func makeChatMessage(role: String, text: String) -> Ai_Agent_Platform_V1_ChatMessage {
        var textBlock = Ai_Agent_Platform_V1_TextBlock()
        textBlock.text = text
        var contentBlock = Ai_Agent_Platform_V1_ContentBlock()
        contentBlock.text = textBlock
        var msg = Ai_Agent_Platform_V1_ChatMessage()
        msg.role = role
        msg.blocks = [contentBlock]
        return msg
    }
}
