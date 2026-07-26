import Foundation
import Vision

struct Request: Decodable {
  let imagePath: String
}

struct Response: Encodable {
  let found: Bool
  let centerX: Double
  let centerY: Double
  let confidence: Double
}

func detect(_ request: Request) throws -> Response {
  let visionRequest = VNDetectFaceRectanglesRequest()
  let handler = VNImageRequestHandler(url: URL(fileURLWithPath: request.imagePath))
  try handler.perform([visionRequest])
  guard let face = (visionRequest.results ?? []).max(by: {
    $0.boundingBox.width * $0.boundingBox.height < $1.boundingBox.width * $1.boundingBox.height
  }) else {
    return Response(found: false, centerX: 0.5, centerY: 0.5, confidence: 0)
  }
  let box = face.boundingBox
  return Response(
    found: true,
    centerX: Double(box.midX),
    centerY: Double(1 - box.midY),
    confidence: Double(face.confidence)
  )
}

do {
  let data = try FileHandle.standardInput.readToEnd() ?? Data()
  let decoder = JSONDecoder()
  decoder.keyDecodingStrategy = .convertFromSnakeCase
  let response = try detect(decoder.decode(Request.self, from: data))
  let encoder = JSONEncoder()
  encoder.keyEncodingStrategy = .convertToSnakeCase
  FileHandle.standardOutput.write(try encoder.encode(response))
} catch {
  fputs("vision-anchor-macos: \(error.localizedDescription)\n", stderr)
  exit(2)
}
