import XCTest

final class AgentDemoUITests: XCTestCase {
    private var mockServer: MockGRPCServer!

    override func setUp() async throws {
        continueAfterFailure = false

        let mockAuth = MockAuthServiceImpl()
        let mockChat = MockChatServiceImpl(
            sessionID: "ui-test-session",
            assistantText: "Hello from assistant"
        )
        mockServer = MockGRPCServer(services: [mockAuth, mockChat])
        try await mockServer.start()
    }

    override func tearDown() {
        mockServer.stop()
        mockServer = nil
    }

    @MainActor
    func testLoginFlow() throws {
        let port = try mockServer.port
        let app = XCUIApplication()
        app.launchArguments = ["--reset-state"]
        app.launchEnvironment = [
            "SERVER_HOST": "127.0.0.1",
            "SERVER_PORT": "\(port)",
            "SERVER_PLAINTEXT": "true",
        ]
        app.launch()

        let emailField = app.textFields["Email"]
        XCTAssertTrue(emailField.waitForExistence(timeout: 5))
        emailField.tap()
        emailField.typeText("test@example.com")

        let passwordField = app.secureTextFields["Password"]
        passwordField.tap()
        passwordField.typeText("password123")

        app.buttons["Sign In"].tap()

        let chatNavBar = app.navigationBars["Chat"]
        XCTAssertTrue(chatNavBar.waitForExistence(timeout: 10))
    }

    @MainActor
    func testSendMessage() throws {
        let port = try mockServer.port
        let app = XCUIApplication()
        app.launchArguments = ["--reset-state"]
        app.launchEnvironment = [
            "SERVER_HOST": "127.0.0.1",
            "SERVER_PORT": "\(port)",
            "SERVER_PLAINTEXT": "true",
        ]
        app.launch()

        // Login first
        let emailField = app.textFields["Email"]
        XCTAssertTrue(emailField.waitForExistence(timeout: 5))
        emailField.tap()
        emailField.typeText("test@example.com")

        let passwordField = app.secureTextFields["Password"]
        passwordField.tap()
        passwordField.typeText("password123")

        app.buttons["Sign In"].tap()

        let chatNavBar = app.navigationBars["Chat"]
        XCTAssertTrue(chatNavBar.waitForExistence(timeout: 10))

        // Send a message
        let messageField = app.textFields["Type a message"]
        XCTAssertTrue(messageField.waitForExistence(timeout: 5))
        messageField.tap()
        messageField.typeText("Hello")

        app.buttons["Send"].tap()

        // Verify user message and assistant reply appear
        let userMessage = app.staticTexts["Hello"]
        XCTAssertTrue(userMessage.waitForExistence(timeout: 5))

        let assistantReply = app.staticTexts["Hello from assistant"]
        XCTAssertTrue(assistantReply.waitForExistence(timeout: 10))
    }
}
