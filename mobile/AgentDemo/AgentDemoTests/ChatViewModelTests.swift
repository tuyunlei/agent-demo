@testable import AgentDemo
import Foundation
import GRPCClient
import Testing

@MainActor
struct ChatViewModelTests {
    @Test func sendMessage_appendsUserAndAssistantMessages() async throws {
        let mockService = MockChatService()
        await mockService.enqueue(result: .success(makeResponse(texts: ["Hello from AI"])))
        let viewModel = ChatViewModel(chatService: mockService)

        await viewModel.sendMessage(text: "Hi", token: "token")

        #expect(viewModel.messages.count == 2)
        #expect(viewModel.messages[0].role == .user)
        #expect(viewModel.messages[0].text == "Hi")
        #expect(viewModel.messages[1].role == .assistant)
        #expect(viewModel.messages[1].text == "Hello from AI")
    }

    @Test func sendMessage_usesFallbackWhenAssistantContentIsEmpty() async throws {
        let mockService = MockChatService()
        await mockService.enqueue(result: .success(makeResponse(texts: [])))
        let viewModel = ChatViewModel(chatService: mockService)

        await viewModel.sendMessage(text: "Hi", token: "token")

        #expect(viewModel.messages.count == 2)
        #expect(viewModel.messages[1].text == "(no response)")
    }

    @Test func sendMessage_setsErrorMessageWhenServiceFails() async throws {
        let mockService = MockChatService()
        await mockService.setShouldFail(true)
        let viewModel = ChatViewModel(chatService: mockService)

        await viewModel.sendMessage(text: "Hi", token: "token")

        #expect(viewModel.errorMessage == "Unable to connect to server. Please check your network and try again.")
        #expect(viewModel.messages.isEmpty)
    }

    @Test func sendMessageFailureRemovesOptimisticMessage() async {
        let mockService = MockChatService()
        await mockService.setShouldFail(true)
        let viewModel = ChatViewModel(chatService: mockService)

        await viewModel.sendMessage(text: "hello", token: "token")

        #expect(viewModel.messages.isEmpty)
        #expect(viewModel.errorMessage != nil)
    }

    @Test func sendMessage_updatesSendingStateDuringRequest() async throws {
        let mockService = MockChatService()
        await mockService.enqueue(result: .success(makeResponse(texts: ["Done"])), delayNanoseconds: 150_000_000)
        let viewModel = ChatViewModel(chatService: mockService)

        let task = Task {
            await viewModel.sendMessage(text: "Hi", token: "token")
        }

        try await Task.sleep(nanoseconds: 30_000_000)
        #expect(viewModel.isSending)

        await task.value
        #expect(!viewModel.isSending)
    }

    @Test func sendMessage_persistsSessionIDFromResponse() async throws {
        let mockService = MockChatService()
        await mockService.enqueue(result: .success(makeResponse(texts: ["OK"], sessionID: "session-123")))
        let viewModel = ChatViewModel(chatService: mockService)

        await viewModel.sendMessage(text: "Hi", token: "token")

        #expect(viewModel.sessionID == "session-123")
    }

    private func makeResponse(texts: [String], sessionID: String = "") -> Ai_Agent_Platform_V1_SendMessageResponse {
        var response = Ai_Agent_Platform_V1_SendMessageResponse()
        response.sessionID = sessionID
        response.assistantContent = texts.map { text in
            var textBlock = Ai_Agent_Platform_V1_TextBlock()
            textBlock.text = text

            var contentBlock = Ai_Agent_Platform_V1_ContentBlock()
            contentBlock.text = textBlock
            return contentBlock
        }
        return response
    }
}

private actor MockChatService: ChatServiceProtocol {
    private var queue: [QueuedResult] = []
    private var shouldFail = false

    func setShouldFail(_ value: Bool) {
        shouldFail = value
    }

    func enqueue(
        result: Result<Ai_Agent_Platform_V1_SendMessageResponse, Error>,
        delayNanoseconds: UInt64 = 0
    ) {
        queue.append(QueuedResult(result: result, delayNanoseconds: delayNanoseconds))
    }

    func sendMessage(
        token _: String,
        requestID _: String,
        text _: String,
        sessionID _: String,
        agentID _: String
    ) async throws -> Ai_Agent_Platform_V1_SendMessageResponse {
        if shouldFail {
            throw MockError.network
        }

        guard !queue.isEmpty else {
            throw MockError.missingStub
        }

        let next = queue.removeFirst()
        if next.delayNanoseconds > 0 {
            try await Task.sleep(nanoseconds: next.delayNanoseconds)
        }

        return try next.result.get()
    }
}

private struct QueuedResult {
    let result: Result<Ai_Agent_Platform_V1_SendMessageResponse, Error>
    let delayNanoseconds: UInt64
}

private enum MockError: LocalizedError {
    case network
    case missingStub

    var errorDescription: String? {
        switch self {
        case .network:
            return "Network error"
        case .missingStub:
            return "Missing stubbed response"
        }
    }
}
