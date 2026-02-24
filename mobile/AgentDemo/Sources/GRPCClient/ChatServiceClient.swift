import GRPCCore
import GRPCProtobuf

public struct ChatServiceClient: ChatServiceProtocol {
    private let apiClient: APIClient

    public init(apiClient: APIClient = APIClient()) {
        self.apiClient = apiClient
    }

    public func sendMessage(
        requestID: String,
        text: String,
        sessionID: String = "",
        agentID: String = ""
    ) async throws -> Ai_Agent_Platform_V1_SendMessageResponse {
        var textBlock = Ai_Agent_Platform_V1_TextBlock()
        textBlock.text = text

        var contentBlock = Ai_Agent_Platform_V1_ContentBlock()
        contentBlock.text = textBlock

        var msg = Ai_Agent_Platform_V1_SendMessageRequest()
        msg.requestID = requestID
        msg.sessionID = sessionID
        msg.agentID = agentID
        msg.content = [contentBlock]
        let request = msg

        return try await apiClient.withAuthenticatedClient { client, token in
            var metadata = Metadata()
            metadata.addString("Bearer \(token)", forKey: "authorization")

            return try await client.unary(
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
