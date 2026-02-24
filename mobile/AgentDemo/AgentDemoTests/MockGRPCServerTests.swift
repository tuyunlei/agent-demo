import GRPCClient
import GRPCCore
import GRPCNIOTransportHTTP2
import Testing

struct MockGRPCServerTests {
    @Test func loginViaMockServer() async throws {
        let mockAuth = MockAuthService(
            accessToken: "test-access-token",
            refreshToken: "test-refresh-token"
        )

        let transport = HTTP2ServerTransport.Posix(
            address: .ipv4(host: "127.0.0.1", port: 0),
            transportSecurity: .plaintext
        )

        var router = RPCRouter<HTTP2ServerTransport.Posix>()
        mockAuth.registerMethods(with: &router)
        let server = GRPCServer(transport: transport, router: router)

        let serverTask = Task { try await server.serve() }
        defer { serverTask.cancel() }

        let address = try await transport.listeningAddress
        guard let port = address.ipv4?.port ?? address.ipv6?.port else {
            Issue.record("Expected IP address, got: \(address)")
            return
        }

        let apiClient = APIClient(
            host: "127.0.0.1",
            port: port,
            usePlaintext: true
        )
        let authClient = AuthServiceClient(apiClient: apiClient)

        let response = try await authClient.login(
            email: "test@example.com",
            password: "password123"
        )

        #expect(response.userID == "mock-user-id")
        #expect(response.tokenPair.accessToken == "test-access-token")
        #expect(response.tokenPair.refreshToken == "test-refresh-token")
    }
}

// MARK: - Mock Auth Service

private struct MockAuthService: Ai_Agent_Platform_V1_AuthService.SimpleServiceProtocol {
    let accessToken: String
    let refreshToken: String

    func login(
        request _: Ai_Agent_Platform_V1_LoginRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_LoginResponse {
        var tokenPair = Ai_Agent_Platform_V1_TokenPair()
        tokenPair.accessToken = accessToken
        tokenPair.refreshToken = refreshToken

        var response = Ai_Agent_Platform_V1_LoginResponse()
        response.userID = "mock-user-id"
        response.tokenPair = tokenPair
        return response
    }

    func register(
        request _: Ai_Agent_Platform_V1_RegisterRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_RegisterResponse {
        throw RPCError(code: .unimplemented, message: "Not implemented in mock")
    }

    func refreshToken(
        request _: Ai_Agent_Platform_V1_RefreshTokenRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_RefreshTokenResponse {
        throw RPCError(code: .unimplemented, message: "Not implemented in mock")
    }

    func logout(
        request _: Ai_Agent_Platform_V1_LogoutRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_LogoutResponse {
        throw RPCError(code: .unimplemented, message: "Not implemented in mock")
    }
}
