// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "AgentDemoPackages",
    platforms: [.iOS("18.0")],
    products: [
        .library(name: "GRPCClient", targets: ["GRPCClient"]),
    ],
    dependencies: [
        .package(url: "https://github.com/grpc/grpc-swift.git", from: "2.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-nio-transport.git", from: "1.0.0"),
        .package(url: "https://github.com/grpc/grpc-swift-protobuf.git", from: "1.0.0"),
        .package(url: "https://github.com/apple/swift-protobuf.git", from: "1.0.0"),
    ],
    targets: [
        .target(
            name: "GRPCClient",
            dependencies: [
                .product(name: "GRPCCore", package: "grpc-swift"),
                .product(name: "GRPCNIOTransportHTTP2", package: "grpc-swift-nio-transport"),
                .product(name: "GRPCNIOTransportHTTP2TransportServices", package: "grpc-swift-nio-transport"),
                .product(name: "GRPCProtobuf", package: "grpc-swift-protobuf"),
                .product(name: "SwiftProtobuf", package: "swift-protobuf"),
            ],
            path: "Sources/GRPCClient",
            plugins: [
                .plugin(name: "GRPCProtobufGenerator", package: "grpc-swift-protobuf"),
            ]
        ),
        .target(
            name: "GRPCTestSupport",
            dependencies: [
                "GRPCClient",
                .product(name: "GRPCCore", package: "grpc-swift"),
                .product(name: "GRPCNIOTransportHTTP2", package: "grpc-swift-nio-transport"),
            ],
            path: "Sources/GRPCTestSupport"
        ),
    ]
)
