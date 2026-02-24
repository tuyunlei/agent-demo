import Combine
import Foundation
import GRPCClient
import Security
import SwiftUI

@MainActor
final class AppState: ObservableObject {
    @Published var accessToken: String? {
        didSet {
            if let accessToken {
                KeychainHelper.save(key: "accessToken", value: accessToken)
            } else {
                KeychainHelper.delete(key: "accessToken")
            }
        }
    }

    @Published var refreshToken: String? {
        didSet {
            if let refreshToken {
                KeychainHelper.save(key: "refreshToken", value: refreshToken)
            } else {
                KeychainHelper.delete(key: "refreshToken")
            }
        }
    }

    let tokenStore: TokenStore

    var isLoggedIn: Bool {
        guard let accessToken else { return false }
        return !accessToken.isEmpty
    }

    init() {
        let isResetState = ProcessInfo.processInfo.arguments.contains("--reset-state")

        if isResetState {
            KeychainHelper.delete(key: "accessToken")
            KeychainHelper.delete(key: "refreshToken")
            UserDefaults.standard.removeObject(forKey: "lastSessionID")
        }

        let loadedAccess = isResetState ? nil : KeychainHelper.load(key: "accessToken")
        let loadedRefresh = isResetState ? nil : KeychainHelper.load(key: "refreshToken")

        let baseClient = APIClient()
        let store = TokenStore(
            accessToken: loadedAccess,
            refreshToken: loadedRefresh,
            refreshHandler: { refreshToken in
                let authClient = AuthServiceClient(apiClient: baseClient)
                let response = try await authClient.refreshToken(token: refreshToken)
                return (response.tokenPair.accessToken, response.tokenPair.refreshToken)
            }
        )
        tokenStore = store
        accessToken = loadedAccess
        refreshToken = loadedRefresh

        store.onTokensUpdated = { [weak self] access, refresh in
            Task { @MainActor [weak self] in
                self?.accessToken = access
                self?.refreshToken = refresh
            }
        }
        store.onAuthExpired = { [weak self] in
            Task { @MainActor [weak self] in
                self?.logout()
            }
        }
    }

    func logout() {
        accessToken = nil
        refreshToken = nil
        UserDefaults.standard.removeObject(forKey: "lastSessionID")
        Task { await tokenStore.clear() }
    }
}
