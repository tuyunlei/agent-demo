import GRPCCore
import GRPCProtobuf

public struct SessionServiceClient: Sendable {
    private let apiClient: APIClient

    public init(apiClient: APIClient = APIClient()) {
        self.apiClient = apiClient
    }

    public func listMessages(
        token: String,
        sessionID: String,
        pageSize: Int32 = 50
    ) async throws -> Ai_Agent_Platform_V1_ListSessionMessagesResponse {
        var pagination = Ai_Agent_Platform_V1_PaginationRequest()
        pagination.pageSize = pageSize

        var request = Ai_Agent_Platform_V1_ListSessionMessagesRequest()
        request.sessionID = sessionID
        request.pagination = pagination

        var metadata = Metadata()
        metadata.addString("Bearer \(token)", forKey: "authorization")

        return try await apiClient.withClient { client in
            try await client.unary(
                request: ClientRequest(message: request, metadata: metadata),
                descriptor: MethodDescriptor(
                    fullyQualifiedService: "ai.agent.platform.v1.SessionService",
                    method: "ListSessionMessages"
                ),
                serializer: ProtobufSerializer<Ai_Agent_Platform_V1_ListSessionMessagesRequest>(),
                deserializer: ProtobufDeserializer<Ai_Agent_Platform_V1_ListSessionMessagesResponse>(),
                options: .defaults,
                onResponse: { try $0.message }
            )
        }
    }
}
