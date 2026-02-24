import GRPCCore
import GRPCNIOTransportHTTP2
import Synchronization

final class MockGRPCServer: Sendable {
    private let services: [any RegistrableRPCService]
    private let task = Mutex<Task<Void, any Error>?>(nil)
    private let _port = Mutex<Int?>(nil)

    var port: Int {
        get throws {
            guard let port = _port.withLock({ $0 }) else {
                throw MockServerError.notStarted
            }
            return port
        }
    }

    init(services: [any RegistrableRPCService]) {
        self.services = services
    }

    func start() async throws {
        let transport = HTTP2ServerTransport.Posix(
            address: .ipv4(host: "127.0.0.1", port: 0),
            transportSecurity: .plaintext
        )

        var router = RPCRouter<HTTP2ServerTransport.Posix>()
        for service in services {
            service.registerMethods(with: &router)
        }
        let server = GRPCServer(transport: transport, router: router)

        let serverTask = Task {
            try await server.serve()
        }
        task.withLock { $0 = serverTask }

        let address = try await transport.listeningAddress
        guard let listeningPort = address.ipv4?.port ?? address.ipv6?.port else {
            throw MockServerError.failedToStart
        }
        _port.withLock { $0 = listeningPort }
    }

    func stop() {
        task.withLock { current in
            current?.cancel()
            current = nil
        }
    }
}

enum MockServerError: Error {
    case notStarted
    case failedToStart
}
