@testable import AgentDemo
import Foundation
import Testing

@MainActor
struct AppStateTests {
    init() {
        UserDefaults.standard.removeObject(forKey: "lastSessionID")
    }

    @Test func isLoggedIn_returnsFalseWhenTokenIsNil() {
        let state = AppState()
        state.accessToken = nil

        #expect(!state.isLoggedIn)
    }

    @Test func isLoggedIn_returnsFalseWhenTokenIsEmpty() {
        let state = AppState()
        state.accessToken = ""

        #expect(!state.isLoggedIn)
    }

    @Test func logout_clearsTokenAndSessionID() {
        let state = AppState()
        state.accessToken = "some-token"
        UserDefaults.standard.set("session-xyz", forKey: "lastSessionID")

        state.logout()

        #expect(state.accessToken == nil)
        #expect(UserDefaults.standard.string(forKey: "lastSessionID") == nil)
    }
}
