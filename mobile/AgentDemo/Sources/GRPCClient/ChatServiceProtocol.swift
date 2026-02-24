import GRPCProtobuf

public protocol ChatServiceProtocol: Sendable {
    func sendMessage(
        requestID: String,
        text: String,
        sessionID: String,
        agentID: String
    ) async throws -> Ai_Agent_Platform_V1_SendMessageResponse
}
