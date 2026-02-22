import Foundation

struct ChatMessage: Identifiable, Equatable {
    enum Role: Equatable {
        case user
        case assistant

        var title: String {
            switch self {
            case .user:
                return "You"
            case .assistant:
                return "Assistant"
            }
        }
    }

    let id = UUID()
    let role: Role
    let text: String
}
