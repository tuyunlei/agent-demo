import GRPCClient
import GRPCCore

public struct MockAuthService: Ai_Agent_Platform_V1_AuthService.SimpleServiceProtocol {
    public let accessToken: String
    public let refreshToken: String

    public init(
        accessToken: String = "mock-access-token",
        refreshToken: String = "mock-refresh-token"
    ) {
        self.accessToken = accessToken
        self.refreshToken = refreshToken
    }

    public func login(
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

    public func register(
        request _: Ai_Agent_Platform_V1_RegisterRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_RegisterResponse {
        throw RPCError(code: .unimplemented, message: "Not implemented in mock")
    }

    public func refreshToken(
        request _: Ai_Agent_Platform_V1_RefreshTokenRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_RefreshTokenResponse {
        throw RPCError(code: .unimplemented, message: "Not implemented in mock")
    }

    public func logout(
        request _: Ai_Agent_Platform_V1_LogoutRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_LogoutResponse {
        throw RPCError(code: .unimplemented, message: "Not implemented in mock")
    }
}
