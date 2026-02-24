//
//  AgentDemoApp.swift
//  AgentDemo
//
//  Created by ByteDance on 2026/2/20.
//

import SwiftUI

@main
struct AgentDemoApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        WindowGroup {
            Group {
                if appState.isLoggedIn {
                    ChatView(tokenStore: appState.tokenStore)
                } else {
                    LoginView()
                }
            }
            .environmentObject(appState)
        }
    }
}
