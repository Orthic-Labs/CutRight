import Foundation
import Speech

struct TimedWord: Codable {
    let s: Double
    let e: Double
    let w: String
}

func fail(_ message: String, code: Int32 = 2) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(code)
}

func write(_ words: [TimedWord], to output: URL) throws {
    let data = try JSONEncoder().encode(words)
    try data.write(to: output, options: .atomic)
}

let arguments = CommandLine.arguments
guard arguments.count == 3 else {
    fail("usage: cutright-verifier <input-audio> <output-json>")
}
let input = URL(fileURLWithPath: arguments[1])
let output = URL(fileURLWithPath: arguments[2])

if arguments[1] == "--self-test" {
    try write([TimedWord(s: 0, e: 0.25, w: "verified")], to: output)
    exit(0)
}
guard FileManager.default.fileExists(atPath: input.path) else {
    fail("input audio is absent")
}

let authorization = DispatchSemaphore(value: 0)
var authorized = false
SFSpeechRecognizer.requestAuthorization { status in
    authorized = status == .authorized
    authorization.signal()
}
guard authorization.wait(timeout: .now() + 10) == .success, authorized else {
    fail("on-device speech verifier is unavailable", code: 3)
}
guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "en-US")),
      recognizer.isAvailable,
      recognizer.supportsOnDeviceRecognition else {
    fail("on-device speech verifier is unsupported", code: 3)
}

let request = SFSpeechURLRecognitionRequest(url: input)
request.requiresOnDeviceRecognition = true
request.shouldReportPartialResults = false
let completion = DispatchSemaphore(value: 0)
var words: [TimedWord] = []
var failure: Error?
let task = recognizer.recognitionTask(with: request) { result, error in
    if let result, result.isFinal {
        words = result.bestTranscription.segments.map { segment in
            TimedWord(
                s: segment.timestamp,
                e: segment.timestamp + segment.duration,
                w: segment.substring
            )
        }
        completion.signal()
    } else if let error {
        failure = error
        completion.signal()
    }
}
guard completion.wait(timeout: .now() + 600) == .success else {
    task.cancel()
    fail("on-device speech verification timed out", code: 4)
}
if let failure {
    fail("on-device speech verification failed: \(failure.localizedDescription)", code: 3)
}
guard !words.isEmpty else {
    fail("on-device speech verifier returned no timed words", code: 3)
}
do {
    try write(words, to: output)
} catch {
    fail("could not write verifier output: \(error.localizedDescription)")
}
