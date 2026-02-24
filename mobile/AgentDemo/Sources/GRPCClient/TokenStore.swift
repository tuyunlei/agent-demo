import Foundation

public enum TokenRefreshError: Error {
    case noRefreshToken
}

public actor TokenStore {
    public typealias RefreshHandler = @Sendable (String) async throws -> (
        access: String, refresh: String
    )

    public private(set) var accessToken: String?
    public private(set) var refreshToken: String?
    private var refreshTask: Task<TokenPair, any Error>?

    private let refreshHandler: RefreshHandler

    /// Set synchronously during setup, before any concurrent access.
    public nonisolated(unsafe) var onTokensUpdated: (
        @Sendable (_ access: String, _ refresh: String) -> Void
    )?
    public nonisolated(unsafe) var onAuthExpired: (@Sendable () -> Void)?

    public init(
        accessToken: String? = nil,
        refreshToken: String? = nil,
        refreshHandler: @escaping RefreshHandler
    ) {
        self.accessToken = accessToken
        self.refreshToken = refreshToken
        self.refreshHandler = refreshHandler
    }

    public func setTokens(access: String, refresh: String) {
        accessToken = access
        refreshToken = refresh
    }

    /// Refresh the access token. Concurrent callers share a single in-flight refresh.
    public func refresh() async throws -> String {
        if let existingTask = refreshTask {
            let pair = try await existingTask.value
            return pair.access
        }

        guard let currentRefreshToken = refreshToken else {
            throw TokenRefreshError.noRefreshToken
        }

        let handler = refreshHandler
        let task = Task<TokenPair, any Error> {
            let result = try await handler(currentRefreshToken)
            return TokenPair(access: result.access, refresh: result.refresh)
        }
        refreshTask = task

        do {
            let pair = try await task.value
            refreshTask = nil
            accessToken = pair.access
            refreshToken = pair.refresh
            onTokensUpdated?(pair.access, pair.refresh)
            return pair.access
        } catch {
            refreshTask = nil
            onAuthExpired?()
            throw error
        }
    }

    public func clear() {
        accessToken = nil
        refreshToken = nil
        refreshTask?.cancel()
        refreshTask = nil
    }
}

private struct TokenPair: Sendable {
    let access: String
    let refresh: String
}
