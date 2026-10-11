import Foundation

nonisolated enum DriveError: LocalizedError, Equatable {
    case invalidName
    case nameTaken(String)
    case intoItself
    case outsideDrive

    var errorDescription: String? {
        switch self {
        case .invalidName: "This name is not allowed. Do not use “/” or “:”, and do not start with a dot."
        case .nameTaken(let name): "The name “\(name)” is already used in this folder."
        case .intoItself: "A folder cannot be moved into itself."
        case .outsideDrive: "Choose a folder inside your Drive."
        }
    }
}
