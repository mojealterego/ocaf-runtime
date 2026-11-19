import Foundation

@main
struct ExportLeaseTest {
    static func main() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: false)
        defer { try? FileManager.default.removeItem(at: root) }
        let lease = LocalExportLease(root: root)
        let first = try lease.begin(data: Data([1, 2]), name: "first.json")
        let saved = try Data(contentsOf: first)
        precondition(saved == Data([1, 2]))
        do { _ = try lease.begin(data: Data([3]), name: "second.json"); fatalError("overlapping export accepted") }
        catch LocalExportLease.Failure.busy {}
        lease.finish(first)
        precondition(!FileManager.default.fileExists(atPath: first.deletingLastPathComponent().path))
        let second = try lease.begin(data: Data([4]), name: "second.json")
        lease.finish(first)
        precondition(FileManager.default.fileExists(atPath: second.path))
        lease.finish(second)
        for invalid in ["..", ".", "../private", "a/b", "", "a\\b"] {
            do { _ = try lease.begin(data: Data([5]), name: invalid); fatalError("invalid filename") }
            catch LocalExportLease.Failure.invalidInput {}
        }
        do { _ = try lease.begin(data: Data(count: 131073), name: "large.json"); fatalError("oversized export") }
        catch LocalExportLease.Failure.invalidInput {}
        precondition(try! FileManager.default.contentsOfDirectory(atPath: root.path).isEmpty)
        print("iOS LocalExportLease: contract checks PASS")
    }
}
