import Foundation
import GRPCCore
import GRPCNIOTransportHTTP2TransportServices

public struct APIClient: Sendable {
    public let host: String
    public let port: Int
    public let usePlaintext: Bool

    public init(
        host: String = ProcessInfo.processInfo.environment["SERVER_HOST"] ?? "localhost",
        port: Int = Int(ProcessInfo.processInfo.environment["SERVER_PORT"] ?? "") ?? 8443,
        usePlaintext: Bool = ProcessInfo.processInfo.environment["SERVER_PLAINTEXT"] == "true"
    ) {
        self.host = host
        self.port = port
        self.usePlaintext = usePlaintext
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
}
