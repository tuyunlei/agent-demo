import Foundation
import GRPCCore
import GRPCNIOTransportHTTP2

public struct APIClient {
    public let host: String
    public let port: Int

    public init(host: String = "REDACTED_HOST", port: Int = 8443) {
        self.host = host
        self.port = port
    }

    public func withClient<T>(
        _ operation: @escaping (any GRPCClient) async throws -> T
    ) async throws -> T {
        try await withGRPCClient(
            transport: .http2NIOTransportServices(
                target: .dns(host: self.host, port: self.port),
                transportSecurity: .tls
            )
        ) { client in
            try await operation(client)
        }
    }
}
