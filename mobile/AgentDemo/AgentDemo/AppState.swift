import Combine
import SwiftUI
import Combine

@MainActor
final class AppState: ObservableObject {
    @Published var accessToken: String?

    var isLoggedIn: Bool {
        guard let accessToken else { return false }
        return !accessToken.isEmpty
    }

    func logout() {
        accessToken = nil
    }
}
