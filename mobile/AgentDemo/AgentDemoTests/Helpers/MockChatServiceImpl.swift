import GRPCClient
import GRPCCore
import Synchronization

final class MockChatServiceImpl: Ai_Agent_Platform_V1_ChatService.SimpleServiceProtocol, Sendable {
    let sessionID: String
    let assistantText: String
    let failUntilCall: Int
    private let _callCount = Mutex<Int>(0)

    var callCount: Int {
        _callCount.withLock { $0 }
    }

    init(
        sessionID: String = "mock-session-id",
        assistantText: String = "Hello from mock assistant",
        failUntilCall: Int = 0
    ) {
        self.sessionID = sessionID
        self.assistantText = assistantText
        self.failUntilCall = failUntilCall
    }

    func sendMessage(
        request _: Ai_Agent_Platform_V1_SendMessageRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_SendMessageResponse {
        let currentCall = _callCount.withLock { count in
            count += 1
            return count
        }

        if failUntilCall > 0, currentCall <= failUntilCall {
            throw RPCError(code: .unauthenticated, message: "Token expired")
        }

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
        // Empty stream
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
