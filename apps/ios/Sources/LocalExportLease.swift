import Foundation

/// Main-thread-confined owner of one share-sheet export in a private directory.
final class LocalExportLease {
    enum Failure: Error { case busy, invalidInput }

    private let root: URL
    private var active: URL?

    init(root: URL = FileManager.default.temporaryDirectory) {
        self.root = root
    }

    func begin(data: Data, name: String) throws -> URL {
        guard active == nil else { throw Failure.busy }
        guard !data.isEmpty, data.count <= 128 * 1024,
              !name.isEmpty, name.utf8.count <= 100, name != ".", name != "..",
              name.utf8.allSatisfy({ (48...57).contains($0) || (65...90).contains($0) ||
                  (97...122).contains($0) || [45, 46, 95].contains($0) }) else {
            throw Failure.invalidInput
        }
        let directory = root.appendingPathComponent("ocaf-export-" + UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false,
                                                attributes: [.posixPermissions: 0o700])
        let file = directory.appendingPathComponent(name, isDirectory: false)
        do {
            try data.write(to: file, options: .atomic)
            try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: file.path)
        } catch {
            try? FileManager.default.removeItem(at: directory)
            throw error
        }
        active = file
        return file
    }

    /// Ignore late completions from an earlier share. Retain the lease on cleanup failure.
    @discardableResult
    func finish(_ file: URL) -> Bool {
        guard active == file else { return false }
        do {
            try FileManager.default.removeItem(at: file.deletingLastPathComponent())
            active = nil
            return true
        } catch {
            return false
        }
    }

    deinit {
        if let file = active { try? FileManager.default.removeItem(at: file.deletingLastPathComponent()) }
    }
}
