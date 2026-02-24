import Foundation
import GRPCCore
import GRPCNIOTransportHTTP2TransportServices

public enum APIClientError: Error {
    case noTokenStore
    case notAuthenticated
}

public struct APIClient: Sendable {
    public let host: String
    public let port: Int
    public let usePlaintext: Bool
    public let tokenStore: TokenStore?

    public init(
        host: String = ProcessInfo.processInfo.environment["SERVER_HOST"] ?? "localhost",
        port: Int = Int(ProcessInfo.processInfo.environment["SERVER_PORT"] ?? "") ?? 8443,
        usePlaintext: Bool = ProcessInfo.processInfo.environment["SERVER_PLAINTEXT"] == "true",
        tokenStore: TokenStore? = nil
    ) {
        self.host = host
        self.port = port
        self.usePlaintext = usePlaintext
        self.tokenStore = tokenStore
    }

    public func withClient<T: Sendable>(
        _ operation: @escaping (GRPCClient<HTTP2ClientTransport.TransportServices>) async throws -> T
    ) async throws -> T {
        let transportSecurity: HTTP2ClientTransport.TransportServices.TransportSecurity =
            usePlaintext ? .plaintext : .tls
        let transport = try HTTP2ClientTransport.TransportServices(
            target: .dns(host: host, port: port),
            transportSecurity: transportSecurity
        )

        return try await withGRPCClient(transport: transport) { client in
            try await operation(client)
        }
    }

    /// Execute an authenticated operation with automatic token refresh on UNAUTHENTICATED.
    public func withAuthenticatedClient<T: Sendable>(
        _ operation: @escaping @Sendable (
            GRPCClient<HTTP2ClientTransport.TransportServices>, String
        ) async throws -> T
    ) async throws -> T {
        guard let tokenStore else { throw APIClientError.noTokenStore }
        guard let token = await tokenStore.accessToken else {
            throw APIClientError.notAuthenticated
        }

        do {
            return try await withClient { client in
                try await operation(client, token)
            }
        } catch let error as RPCError where error.code == .unauthenticated {
            let newToken = try await tokenStore.refresh()
            return try await withClient { client in
                try await operation(client, newToken)
            }
        }
    }
}
