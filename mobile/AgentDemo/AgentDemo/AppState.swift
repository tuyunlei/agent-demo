import Combine
import Foundation
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

    var isLoggedIn: Bool {
        guard let accessToken else { return false }
        return !accessToken.isEmpty
    }

    init() {
        if ProcessInfo.processInfo.arguments.contains("--reset-state") {
            KeychainHelper.delete(key: "accessToken")
            UserDefaults.standard.removeObject(forKey: "lastSessionID")
            accessToken = nil
            return
        }
        accessToken = KeychainHelper.load(key: "accessToken")
    }

    func logout() {
        accessToken = nil
        UserDefaults.standard.removeObject(forKey: "lastSessionID")
    }
}
