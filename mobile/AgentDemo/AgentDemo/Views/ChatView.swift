import SwiftUI

struct ChatView: View {
    @EnvironmentObject private var appState: AppState

    @StateObject private var viewModel = ChatViewModel()
    @State private var inputText = ""

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                if let errorMessage = viewModel.errorMessage {
                    HStack {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundStyle(.yellow)
                        Text(errorMessage)
                            .font(.caption)
                        Spacer()
                        Button("Dismiss") {
                            viewModel.errorMessage = nil
                        }
                        .font(.caption)
                    }
                    .padding(8)
                    .background(Color.red.opacity(0.1))
                    .cornerRadius(8)
                    .padding(.horizontal)
                    .padding(.bottom, 8)
                }

                if viewModel.isLoadingHistory {
                    ProgressView("Loading history...")
                        .padding()
                }

                List(viewModel.messages) { message in
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

                HStack(spacing: 8) {
                    TextField("Type a message", text: $inputText, axis: .vertical)
                        .textFieldStyle(.roundedBorder)
                        .lineLimit(1 ... 4)

                    let isInputEmpty = inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                    Button("Send", action: sendMessage)
                        .disabled(viewModel.isSending || isInputEmpty)
                }
                .padding()
            }
            .navigationTitle("Chat")
            .toolbar {
                ToolbarItem(placement: .topBarTrailing) {
                    Button("Sign Out") {
                        appState.logout()
                    }
                }
            }
            .task {
                if let token = appState.accessToken {
                    await viewModel.loadHistory(token: token)
                }
            }
        }
    }

    private func sendMessage() {
        let text = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty, let token = appState.accessToken else { return }

        inputText = ""

        Task {
            await viewModel.sendMessage(text: text, token: token)
        }
    }
}
