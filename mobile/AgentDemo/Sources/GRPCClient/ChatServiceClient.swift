import GRPCCore
import GRPCProtobuf

public struct ChatServiceClient {
    private let apiClient: APIClient

    public init(apiClient: APIClient = APIClient()) {
        self.apiClient = apiClient
    }

    public func sendMessage(
        token: String,
        requestID: String,
        text: String,
        sessionID: String = "",
        agentID: String = ""
    ) async throws -> Ai_Agent_Platform_V1_SendMessageResponse {
        var textBlock = Ai_Agent_Platform_V1_TextBlock()
        textBlock.text = text

        var contentBlock = Ai_Agent_Platform_V1_ContentBlock()
        contentBlock.text = textBlock

        var request = Ai_Agent_Platform_V1_SendMessageRequest()
        request.requestID = requestID
        request.sessionID = sessionID
        request.agentID = agentID
        request.content = [contentBlock]

        var metadata = Metadata()
        metadata.addString("Bearer \(token)", forKey: "authorization")

        return try await apiClient.withClient { client in
            try await client.unary(
                request: ClientRequest(message: request, metadata: metadata),
                descriptor: MethodDescriptor(
                    fullyQualifiedService: "ai.agent.platform.v1.ChatService",
                    method: "SendMessage"
                ),
                serializer: ProtobufSerializer<Ai_Agent_Platform_V1_SendMessageRequest>(),
                deserializer: ProtobufDeserializer<Ai_Agent_Platform_V1_SendMessageResponse>(),
                options: .defaults,
                onResponse: { try $0.message }
            )
        }
    }
}
