import GRPCClient
import SwiftUI

struct ChatView: View {
    @EnvironmentObject private var appState: AppState

    @State private var inputText = ""
    @State private var messages: [ChatMessage] = []
    @State private var isSending = false
    @State private var errorMessage: String?
    @State private var sessionID = ""

    private let chatClient = ChatServiceClient()

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                List(messages) { message in
                    HStack {
                        if message.role == .assistant { Spacer(minLength: 40) }

                        VStack(alignment: .leading, spacing: 4) {
                            Text(message.role.title)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Text(message.text)
                        }
                        .padding(10)
                        .background(message.role == .user ? Color.blue.opacity(0.15) : Color.gray.opacity(0.15))
                        .clipShape(RoundedRectangle(cornerRadius: 10))

                        if message.role == .user { Spacer(minLength: 40) }
                    }
                    .listRowSeparator(.hidden)
                }
                .listStyle(.plain)

                if let errorMessage {
                    Text(errorMessage)
                        .foregroundStyle(.red)
                        .font(.footnote)
                        .padding(.horizontal)
                        .padding(.bottom, 8)
                }

                HStack(spacing: 8) {
                    TextField("Type a message", text: $inputText, axis: .vertical)
                        .textFieldStyle(.roundedBorder)
                        .lineLimit(1 ... 4)

                    Button("Send", action: sendMessage)
                        .disabled(isSending || inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
                .padding()
            }
            .navigationTitle("Chat")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Logout") {
                        appState.logout()
                    }
                }
            }
        }
    }

    private func sendMessage() {
        let text = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty, let token = appState.accessToken else { return }

        inputText = ""
        errorMessage = nil
        isSending = true
        messages.append(ChatMessage(role: .user, text: text))

        Task {
            defer { isSending = false }
            do {
                let response = try await chatClient.sendMessage(
                    token: token,
                    requestID: UUID().uuidString,
                    text: text,
                    sessionID: sessionID
                )

                if !response.sessionID.isEmpty {
                    sessionID = response.sessionID
                }

                let assistantText = response.assistantContent
                    .compactMap { block in block.text.text }
                    .joined()
                messages.append(ChatMessage(role: .assistant, text: assistantText))
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}

private struct ChatMessage: Identifiable {
    enum Role {
        case user
        case assistant

        var title: String {
            switch self {
            case .user: return "You"
            case .assistant: return "Assistant"
            }
        }
    }

    let id = UUID()
    let role: Role
    let text: String
}
