import GRPCProtobuf

public protocol SessionServiceProtocol: Sendable {
    func listMessages(
        token: String,
        sessionID: String,
        pageSize: Int32
    ) async throws -> Ai_Agent_Platform_V1_ListSessionMessagesResponse
}

public extension SessionServiceProtocol {
    func listMessages(
        token: String,
        sessionID: String
    ) async throws -> Ai_Agent_Platform_V1_ListSessionMessagesResponse {
        try await listMessages(token: token, sessionID: sessionID, pageSize: 50)
    }
}
