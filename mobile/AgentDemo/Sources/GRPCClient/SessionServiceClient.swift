import GRPCCore
import GRPCProtobuf

public struct SessionServiceClient: SessionServiceProtocol {
    private let apiClient: APIClient

    public init(apiClient: APIClient = APIClient()) {
        self.apiClient = apiClient
    }

    public func listMessages(
        sessionID: String,
        pageSize: Int32 = 50
    ) async throws -> Ai_Agent_Platform_V1_ListSessionMessagesResponse {
        var pagination = Ai_Agent_Platform_V1_PaginationRequest()
        pagination.pageSize = pageSize

        var msg = Ai_Agent_Platform_V1_ListSessionMessagesRequest()
        msg.sessionID = sessionID
        msg.pagination = pagination
        let request = msg

        return try await apiClient.withAuthenticatedClient { client, token in
            var metadata = Metadata()
            metadata.addString("Bearer \(token)", forKey: "authorization")

            return try await client.unary(
                request: ClientRequest(message: request, metadata: metadata),
                descriptor: MethodDescriptor(
                    fullyQualifiedService: "ai.agent.platform.v1.SessionService",
                    method: "ListSessionMessages"
                ),
                serializer: ProtobufSerializer<
                    Ai_Agent_Platform_V1_ListSessionMessagesRequest
                >(),
                deserializer: ProtobufDeserializer<
                    Ai_Agent_Platform_V1_ListSessionMessagesResponse
                >(),
                options: .defaults,
                onResponse: { try $0.message }
            )
        }
    }
}
