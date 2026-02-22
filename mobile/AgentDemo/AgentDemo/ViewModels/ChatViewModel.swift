import Combine
import GRPCClient
import SwiftUI

@MainActor
final class ChatViewModel: ObservableObject {
    @Published var messages: [ChatMessage] = []
    @Published var isSending = false
    @Published var errorMessage: String?

    private(set) var sessionID = ""
    private let chatService: ChatServiceProtocol

    init(chatService: ChatServiceProtocol = ChatServiceClient()) {
        self.chatService = chatService
    }

    func sendMessage(text: String, token: String) async {
        let trimmedText = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedText.isEmpty else { return }

        errorMessage = nil
        isSending = true
        messages.append(ChatMessage(role: .user, text: trimmedText))

        do {
            let response = try await chatService.sendMessage(
                token: token,
                requestID: UUID().uuidString,
                text: trimmedText,
                sessionID: sessionID,
                agentID: ""
            )
            updateSessionID(from: response)
            appendAssistantMessage(from: response)
        } catch {
            if !messages.isEmpty {
                messages.removeLast()
            }
            errorMessage = friendlyError(from: error)
        }

        isSending = false
    }

    private func friendlyError(from error: Error) -> String {
        let description = error.localizedDescription.lowercased()

        let isNetworkError = description.contains("could not connect")
            || description.contains("network")
            || description.contains("timed out")

        if isNetworkError {
            return "Unable to connect to server. Please check your network and try again."
        }

        if description.contains("unauthenticated") || description.contains("401") {
            return "Your session has expired. Please sign in again."
        }

        return "Something went wrong: \(error.localizedDescription)"
    }

    private func updateSessionID(from response: Ai_Agent_Platform_V1_SendMessageResponse) {
        guard !response.sessionID.isEmpty else { return }
        sessionID = response.sessionID
    }

    private func appendAssistantMessage(from response: Ai_Agent_Platform_V1_SendMessageResponse) {
        let textBlocks = response.assistantContent.compactMap { block -> String? in
            switch block.kind {
            case let .text(textBlock):
                return textBlock.text
            default:
                return nil
            }
        }

        let assistantText = textBlocks.joined(separator: "\n")
        let displayText = assistantText.isEmpty ? "(no response)" : assistantText
        messages.append(ChatMessage(role: .assistant, text: displayText))
    }
}
