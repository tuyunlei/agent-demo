import GRPCCore
import GRPCNIOTransportHTTP2TransportServices

public struct APIClient {
    public let host: String
    public let port: Int

    public init(host: String = "REDACTED_HOST", port: Int = 8443) {
        self.host = host
        self.port = port
    }

    public func withClient<T: Sendable>(
        _ operation: @escaping (GRPCClient<HTTP2ClientTransport.TransportServices>) async throws -> T
    ) async throws -> T {
        let transport = try HTTP2ClientTransport.TransportServices(
            target: .dns(host: self.host, port: self.port),
            transportSecurity: .tls
        )

        return try await withGRPCClient(transport: transport) { client in
            try await operation(client)
        }
    }
}
