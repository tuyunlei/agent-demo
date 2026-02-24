import GRPCCore
import GRPCProtobuf

public struct AuthServiceClient: Sendable {
    private let apiClient: APIClient

    public init(apiClient: APIClient = APIClient()) {
        self.apiClient = apiClient
    }

    public func login(
        email: String,
        password: String
    ) async throws -> Ai_Agent_Platform_V1_LoginResponse {
        var request = Ai_Agent_Platform_V1_LoginRequest()
        request.email = email
        request.password = password
        request.deviceName = "iOS"
        request.platform = "ios"

        return try await apiClient.withClient { client in
            try await client.unary(
                request: ClientRequest(message: request),
                descriptor: MethodDescriptor(
                    fullyQualifiedService: "ai.agent.platform.v1.AuthService",
                    method: "Login"
                ),
                serializer: ProtobufSerializer<Ai_Agent_Platform_V1_LoginRequest>(),
                deserializer: ProtobufDeserializer<Ai_Agent_Platform_V1_LoginResponse>(),
                options: .defaults,
                onResponse: { try $0.message }
            )
        }
    }

    public func register(
        email: String,
        password: String,
        displayName: String
    ) async throws -> Ai_Agent_Platform_V1_RegisterResponse {
        var request = Ai_Agent_Platform_V1_RegisterRequest()
        request.email = email
        request.password = password
        request.displayName = displayName

        return try await apiClient.withClient { client in
            try await client.unary(
                request: ClientRequest(message: request),
                descriptor: MethodDescriptor(
                    fullyQualifiedService: "ai.agent.platform.v1.AuthService",
                    method: "Register"
                ),
                serializer: ProtobufSerializer<Ai_Agent_Platform_V1_RegisterRequest>(),
                deserializer: ProtobufDeserializer<Ai_Agent_Platform_V1_RegisterResponse>(),
                options: .defaults,
                onResponse: { try $0.message }
            )
        }
    }

    public func refreshToken(
        token: String
    ) async throws -> Ai_Agent_Platform_V1_RefreshTokenResponse {
        var request = Ai_Agent_Platform_V1_RefreshTokenRequest()
        request.refreshToken = token

        return try await apiClient.withClient { client in
            try await client.unary(
                request: ClientRequest(message: request),
                descriptor: MethodDescriptor(
                    fullyQualifiedService: "ai.agent.platform.v1.AuthService",
                    method: "RefreshToken"
                ),
                serializer: ProtobufSerializer<Ai_Agent_Platform_V1_RefreshTokenRequest>(),
                deserializer: ProtobufDeserializer<Ai_Agent_Platform_V1_RefreshTokenResponse>(),
                options: .defaults,
                onResponse: { try $0.message }
            )
        }
    }
}
