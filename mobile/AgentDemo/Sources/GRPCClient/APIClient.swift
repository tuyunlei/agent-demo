import GRPCCore
import GRPCNIOTransportHTTP2
import GRPCProtobuf

public struct APIClient {
    public static let serverHost = "REDACTED_HOST"
    public static let serverPort = 8443

    public init() {}

    /// Walking skeleton placeholder for gRPC over TLS channel setup.
    ///
    /// grpc-swift v2 + NIO transport is wired as dependencies; concrete
    /// connection lifecycle management (bootstrap, pooling, shutdown) can be
    /// layered on top in follow-up tasks.
    public static func makeTLSChannelDescription() -> String {
        "grpc://\(serverHost):\(serverPort) with TLS (implementation TBD)"
    }
}
