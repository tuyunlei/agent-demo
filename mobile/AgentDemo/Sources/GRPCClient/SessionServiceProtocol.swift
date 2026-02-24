import GRPCProtobuf

public protocol SessionServiceProtocol: Sendable {
    func listMessages(
        sessionID: String,
        pageSize: Int32
    ) async throws -> Ai_Agent_Platform_V1_ListSessionMessagesResponse
}

public extension SessionServiceProtocol {
    func listMessages(
        sessionID: String
    ) async throws -> Ai_Agent_Platform_V1_ListSessionMessagesResponse {
        try await listMessages(sessionID: sessionID, pageSize: 50)
    }
}
