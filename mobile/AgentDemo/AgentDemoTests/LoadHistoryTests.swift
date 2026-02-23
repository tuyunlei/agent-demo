@testable import AgentDemo
import Foundation
import GRPCClient
import Testing

@MainActor
struct LoadHistoryTests {
    init() {
        UserDefaults.standard.removeObject(forKey: "lastSessionID")
    }

    @Test func loadHistory_populatesMessagesFromResponse() async {
        let mockSession = MockSessionService()
        await mockSession.enqueue(result: .success(makeHistoryResponse(messages: [
            ("user", "Hello"),
            ("assistant", "Hi there!"),
        ])))

        let viewModel = makeChatViewModel(sessionClient: mockSession, sessionID: "s-1")

        await viewModel.loadHistory(token: "token")

        #expect(viewModel.messages.count == 2)
        #expect(viewModel.messages[0].role == .user)
        #expect(viewModel.messages[0].text == "Hello")
        #expect(viewModel.messages[1].role == .assistant)
        #expect(viewModel.messages[1].text == "Hi there!")
    }

    @Test func loadHistory_skipsWhenSessionIDIsEmpty() async {
        let mockSession = MockSessionService()
        let viewModel = makeChatViewModel(sessionClient: mockSession, sessionID: "")

        await viewModel.loadHistory(token: "token")

        #expect(viewModel.messages.isEmpty)
        #expect(!viewModel.isLoadingHistory)
    }

    @Test func loadHistory_skipsWhenMessagesNotEmpty() async {
        let mockSession = MockSessionService()
        await mockSession.enqueue(result: .success(makeHistoryResponse(messages: [
            ("user", "First"),
        ])))
        let viewModel = makeChatViewModel(sessionClient: mockSession, sessionID: "s-1")

        // First call loads history
        await viewModel.loadHistory(token: "token")
        #expect(viewModel.messages.count == 1)

        // Second call should skip because messages are not empty
        await viewModel.loadHistory(token: "token")
        #expect(await mockSession.callCount == 1)
    }

    @Test func loadHistory_silentlyHandlesError() async {
        let mockSession = MockSessionService()
        await mockSession.setShouldFail(true)
        let viewModel = makeChatViewModel(sessionClient: mockSession, sessionID: "s-1")

        await viewModel.loadHistory(token: "token")

        #expect(viewModel.messages.isEmpty)
        #expect(viewModel.errorMessage == nil)
        #expect(!viewModel.isLoadingHistory)
    }

    // MARK: - Helpers

    private func makeChatViewModel(
        sessionClient: SessionServiceProtocol,
        sessionID: String
    ) -> ChatViewModel {
        if !sessionID.isEmpty {
            UserDefaults.standard.set(sessionID, forKey: "lastSessionID")
        }
        let mockChat = StubChatService()
        return ChatViewModel(chatService: mockChat, sessionClient: sessionClient)
    }

    private func makeHistoryResponse(
        messages: [(String, String)]
    ) -> Ai_Agent_Platform_V1_ListSessionMessagesResponse {
        var response = Ai_Agent_Platform_V1_ListSessionMessagesResponse()
        response.messages = messages.map { role, text in
            var textBlock = Ai_Agent_Platform_V1_TextBlock()
            textBlock.text = text

            var contentBlock = Ai_Agent_Platform_V1_ContentBlock()
            contentBlock.text = textBlock

            var msg = Ai_Agent_Platform_V1_ChatMessage()
            msg.role = role
            msg.blocks = [contentBlock]
            return msg
        }
        return response
    }
}

// MARK: - Mock Session Service

private actor MockSessionService: SessionServiceProtocol {
    private var queue: [Result<Ai_Agent_Platform_V1_ListSessionMessagesResponse, Error>] = []
    private var shouldFail = false
    private(set) var callCount = 0

    func setShouldFail(_ value: Bool) {
        shouldFail = value
    }

    func enqueue(result: Result<Ai_Agent_Platform_V1_ListSessionMessagesResponse, Error>) {
        queue.append(result)
    }

    func listMessages(
        token _: String,
        sessionID _: String,
        pageSize _: Int32
    ) async throws -> Ai_Agent_Platform_V1_ListSessionMessagesResponse {
        callCount += 1

        if shouldFail {
            throw SessionMockError.network
        }

        guard !queue.isEmpty else {
            throw SessionMockError.missingStub
        }

        return try queue.removeFirst().get()
    }
}

/// Minimal chat service stub that never gets called in loadHistory tests.
private actor StubChatService: ChatServiceProtocol {
    func sendMessage(
        token _: String,
        requestID _: String,
        text _: String,
        sessionID _: String,
        agentID _: String
    ) async throws -> Ai_Agent_Platform_V1_SendMessageResponse {
        throw SessionMockError.missingStub
    }
}

private enum SessionMockError: LocalizedError {
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
