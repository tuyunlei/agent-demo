import GRPCClient
import GRPCCore

struct MockAuthServiceImpl: Ai_Agent_Platform_V1_AuthService.SimpleServiceProtocol {
    let accessToken: String
    let refreshToken: String
    let userID: String
    let loginShouldFail: Bool
    let refreshShouldFail: Bool

    init(
        accessToken: String = "mock-access-token",
        refreshToken: String = "mock-refresh-token",
        userID: String = "mock-user-id",
        loginShouldFail: Bool = false,
        refreshShouldFail: Bool = false
    ) {
        self.accessToken = accessToken
        self.refreshToken = refreshToken
        self.userID = userID
        self.loginShouldFail = loginShouldFail
        self.refreshShouldFail = refreshShouldFail
    }

    func login(
        request _: Ai_Agent_Platform_V1_LoginRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_LoginResponse {
        if loginShouldFail {
            throw RPCError(code: .unauthenticated, message: "Invalid credentials")
        }
        var tokenPair = Ai_Agent_Platform_V1_TokenPair()
        tokenPair.accessToken = accessToken
        tokenPair.refreshToken = refreshToken

        var response = Ai_Agent_Platform_V1_LoginResponse()
        response.userID = userID
        response.tokenPair = tokenPair
        return response
    }

    func register(
        request _: Ai_Agent_Platform_V1_RegisterRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_RegisterResponse {
        var tokenPair = Ai_Agent_Platform_V1_TokenPair()
        tokenPair.accessToken = accessToken
        tokenPair.refreshToken = refreshToken

        var response = Ai_Agent_Platform_V1_RegisterResponse()
        response.userID = userID
        response.tokenPair = tokenPair
        return response
    }

    func refreshToken(
        request _: Ai_Agent_Platform_V1_RefreshTokenRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_RefreshTokenResponse {
        if refreshShouldFail {
            throw RPCError(code: .unauthenticated, message: "Refresh token expired")
        }
        var tokenPair = Ai_Agent_Platform_V1_TokenPair()
        tokenPair.accessToken = "refreshed-access-token"
        tokenPair.refreshToken = "refreshed-refresh-token"

        var response = Ai_Agent_Platform_V1_RefreshTokenResponse()
        response.tokenPair = tokenPair
        return response
    }

    func logout(
        request _: Ai_Agent_Platform_V1_LogoutRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_LogoutResponse {
        var response = Ai_Agent_Platform_V1_LogoutResponse()
        response.success = true
        return response
    }
}
