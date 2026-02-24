import GRPCClient
import GRPCCore

struct MockSessionServiceImpl: Ai_Agent_Platform_V1_SessionService.SimpleServiceProtocol {
    let messages: [Ai_Agent_Platform_V1_ChatMessage]

    init(messages: [Ai_Agent_Platform_V1_ChatMessage] = []) {
        self.messages = messages
    }

    func createSession(
        request _: Ai_Agent_Platform_V1_CreateSessionRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_CreateSessionResponse {
        Ai_Agent_Platform_V1_CreateSessionResponse()
    }

    func getSession(
        request _: Ai_Agent_Platform_V1_GetSessionRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_GetSessionResponse {
        Ai_Agent_Platform_V1_GetSessionResponse()
    }

    func listSessions(
        request _: Ai_Agent_Platform_V1_ListSessionsRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_ListSessionsResponse {
        Ai_Agent_Platform_V1_ListSessionsResponse()
    }

    func listSessionMessages(
        request _: Ai_Agent_Platform_V1_ListSessionMessagesRequest,
        context _: ServerContext
    ) async throws -> Ai_Agent_Platform_V1_ListSessionMessagesResponse {
        var response = Ai_Agent_Platform_V1_ListSessionMessagesResponse()
        response.messages = messages
        return response
    }
}
