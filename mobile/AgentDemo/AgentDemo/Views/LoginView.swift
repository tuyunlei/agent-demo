import GRPCClient
import SwiftUI

struct LoginView: View {
    @EnvironmentObject private var appState: AppState

    @State private var email = ""
    @State private var password = ""
    @State private var isLoading = false
    @State private var errorMessage: String?

    private let authClient = AuthServiceClient()

    var body: some View {
        NavigationStack {
            Form {
                Section("Login") {
                    TextField("Email", text: $email)
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                        .keyboardType(.emailAddress)

                    SecureField("Password", text: $password)
                }

                Section {
                    Button(action: login) {
                        if isLoading {
                            ProgressView()
                                .frame(maxWidth: .infinity)
                        } else {
                            Text("Sign In")
                                .frame(maxWidth: .infinity)
                        }
                    }
                    .disabled(isLoading || email.isEmpty || password.isEmpty)
                }

                if let errorMessage {
                    Section {
                        Text(errorMessage)
                            .foregroundStyle(.red)
                    }
                }
            }
            .navigationTitle("Agent Demo")
        }
    }

    private func login() {
        errorMessage = nil
        isLoading = true

        Task {
            defer { isLoading = false }
            do {
                let response = try await authClient.login(email: email, password: password)
                let token = response.tokenPair.accessToken
                guard !token.isEmpty else {
                    errorMessage = "Login succeeded but access token is empty."
                    return
                }
                appState.accessToken = token
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}
