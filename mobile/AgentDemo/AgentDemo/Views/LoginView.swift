import GRPCClient
import SwiftUI

struct LoginView: View {
    @EnvironmentObject private var appState: AppState

    @State private var email = ""
    @State private var password = ""
    @State private var displayName = ""
    @State private var isRegistering = false
    @State private var isLoading = false
    @State private var errorMessage: String?

    private let authClient = AuthServiceClient()

    var body: some View {
        NavigationStack {
            Form {
                Section("Credentials") {
                    if isRegistering {
                        TextField("Display Name", text: $displayName)
                    }

                    TextField("Email", text: $email)
                        .autocorrectionDisabled()
                        .textInputAutocapitalization(.never)
                        .keyboardType(.emailAddress)

                    SecureField("Password", text: $password)
                }

                Section {
                    Button(action: isRegistering ? register : login) {
                        if isLoading {
                            ProgressView()
                                .frame(maxWidth: .infinity)
                        } else {
                            Text(isRegistering ? "Sign Up" : "Sign In")
                                .frame(maxWidth: .infinity)
                        }
                    }
                    .disabled(isLoading || email.isEmpty || password.isEmpty || (isRegistering && displayName.isEmpty))
                }

                Section {
                    Button(isRegistering ? "Already have an account? Sign In" : "Don't have an account? Sign Up") {
                        isRegistering.toggle()
                        errorMessage = nil
                    }
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
                let pair = response.tokenPair
                guard !pair.accessToken.isEmpty else {
                    errorMessage = "Login succeeded but access token is empty."
                    return
                }
                appState.accessToken = pair.accessToken
                appState.refreshToken = pair.refreshToken
                await appState.tokenStore.setTokens(
                    access: pair.accessToken,
                    refresh: pair.refreshToken
                )
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func register() {
        errorMessage = nil
        isLoading = true

        Task {
            defer { isLoading = false }
            do {
                let response = try await authClient.register(
                    email: email,
                    password: password,
                    displayName: displayName
                )
                let pair = response.tokenPair
                guard !pair.accessToken.isEmpty else {
                    errorMessage = "Sign up succeeded but access token is empty."
                    return
                }
                appState.accessToken = pair.accessToken
                appState.refreshToken = pair.refreshToken
                await appState.tokenStore.setTokens(
                    access: pair.accessToken,
                    refresh: pair.refreshToken
                )
            } catch {
                let localizedDescription = error.localizedDescription.lowercased()
                if localizedDescription.contains("email"), localizedDescription.contains("exist") {
                    errorMessage = "This email is already registered. Please sign in instead."
                } else {
                    errorMessage = error.localizedDescription
                }
            }
        }
    }
}
