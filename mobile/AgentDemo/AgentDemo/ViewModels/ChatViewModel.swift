import Combine
import GRPCClient
import SwiftUI

@MainActor
final class ChatViewModel: ObservableObject {
    @Published var messages: [ChatMessage] = []
    @Published var isSending = false
    @Published var isLoadingHistory = false
    @Published var errorMessage: String?

    private(set) var sessionID: String = "" {
        didSet {
            UserDefaults.standard.set(sessionID, forKey: "lastSessionID")
        }
    }

    private let chatService: ChatServiceProtocol
    private let sessionClient = SessionServiceClient()

    init(chatService: ChatServiceProtocol = ChatServiceClient()) {
        self.chatService = chatService
        sessionID = UserDefaults.standard.string(forKey: "lastSessionID") ?? ""
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

    func loadHistory(token: String) async {
        guard !sessionID.isEmpty else { return }
        guard messages.isEmpty else { return }

        isLoadingHistory = true
        defer { isLoadingHistory = false }

        do {
            let response = try await sessionClient.listMessages(
                token: token,
                sessionID: sessionID
            )

            messages = response.messages.map { protoMsg in
                let role: ChatMessage.Role = protoMsg.role == "user" ? .user : .assistant
                let text = protoMsg.blocks.compactMap { block -> String? in
                    switch block.kind {
                    case let .text(textBlock):
                        return textBlock.text
                    default:
                        return nil
                    }
                }.joined(separator: "\n")

                return ChatMessage(role: role, text: text.isEmpty ? "(empty)" : text)
            }
        } catch {
            // History loading failure is non-fatal, don't show error.
        }
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
