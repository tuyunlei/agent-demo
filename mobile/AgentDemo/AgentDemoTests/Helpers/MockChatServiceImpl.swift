import GRPCClient
import GRPCCore

struct MockChatServiceImpl: Ai_Agent_Platform_V1_ChatService.SimpleServiceProtocol {
    let sessionID: String
    let assistantText: String

    init(
        sessionID: String = "mock-session-id",
        assistantText: String = "Hello from mock assistant"
    ) {
        self.sessionID = sessionID
        self.assistantText = assistantText
    }

    func sendMessage(
        request _: Ai_Agent_Platform_V1_SendMessageRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_SendMessageResponse {
        var textBlock = Ai_Agent_Platform_V1_TextBlock()
        textBlock.text = assistantText

        var contentBlock = Ai_Agent_Platform_V1_ContentBlock()
        contentBlock.text = textBlock

        var response = Ai_Agent_Platform_V1_SendMessageResponse()
        response.sessionID = sessionID
        response.assistantContent = [contentBlock]
        return response
    }

    func subscribe(
        request _: Ai_Agent_Platform_V1_SubscribeRequest,
        response _: RPCWriter<Ai_Agent_Platform_V1_ChatEvent>,
        context _: ServerContext
    ) async throws {
        // Empty stream — return immediately
    }

    func submitToolResult(
        request _: Ai_Agent_Platform_V1_SubmitToolResultRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_SubmitToolResultResponse {
        var response = Ai_Agent_Platform_V1_SubmitToolResultResponse()
        response.accepted = true
        return response
    }
}
