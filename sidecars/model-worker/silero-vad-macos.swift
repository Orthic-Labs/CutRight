import CoreML
import Foundation

struct Request: Decodable {
  let audioPath: String
  let modelPath: String
  let threshold: Float
  let sampleRate: Int
  let minSpeechMs: Int
  let minSilenceMs: Int
}

struct Region: Encodable {
  let startMs: Int
  let endMs: Int
  let meanProbability: Float
}

struct Response: Encodable {
  let provider: String
  let sampleRate: Int
  let regions: [Region]
}

struct RegionOptions {
  let minimumFrames: Int
  let sampleRate: Int
}

enum WorkerError: Error, LocalizedError {
  case invalidRequest(String)
  case model(String)
  case audio(String)

  var errorDescription: String? {
    switch self {
    case .invalidRequest(let message), .model(let message), .audio(let message):
      return message
    }
  }
}

let frameSamples = 512
let contextSamples = 64
let stateCount = 256

func multiArray(_ values: [Float], shape: [NSNumber]) throws -> MLMultiArray {
  let array = try MLMultiArray(shape: shape, dataType: .float32)
  for index in values.indices { array[index] = NSNumber(value: values[index]) }
  return array
}

func floats(_ array: MLMultiArray) -> [Float] {
  (0..<array.count).map { array[$0].floatValue }
}

func score(
  model: MLModel,
  frame: [Float],
  context: inout [Float],
  state: inout [Float]
) throws -> Float {
  let normalized = frame.map { min(1.0, max(-1.0, $0)) }
  let input = try multiArray(
    context + normalized,
    shape: [1, NSNumber(value: contextSamples + frameSamples)]
  )
  let inputState = try multiArray(state, shape: [2, 1, 128])
  let provider = try MLDictionaryFeatureProvider(dictionary: ["input": input, "state_in": inputState])
  let output = try model.prediction(from: provider)
  guard let probability = output.featureValue(for: "output")?.multiArrayValue,
        let outputState = output.featureValue(for: "state_out")?.multiArrayValue else {
    throw WorkerError.model("Silero model did not return output/state_out")
  }
  state = floats(outputState)
  guard state.count == stateCount else {
    throw WorkerError.model("Silero returned invalid recurrent state")
  }
  context = Array(frame.suffix(contextSamples))
  return probability[0].floatValue
}

func completedRegion(
  start: Int,
  end: Int,
  scores: [Float],
  minimumFrames: Int,
  sampleRate: Int
) -> Region? {
  let speechFrames = (end - start) / frameSamples
  guard speechFrames >= minimumFrames, !scores.isEmpty else { return nil }
  return Region(
    startMs: start * 1000 / sampleRate,
    endMs: end * 1000 / sampleRate,
    meanProbability: scores.reduce(0, +) / Float(scores.count)
  )
}

func inputSamples(for request: Request) throws -> [Float] {
  let bytes = try Data(contentsOf: URL(fileURLWithPath: request.audioPath))
  guard bytes.count % MemoryLayout<Float>.size == 0 else {
    throw WorkerError.audio("PCM input is not f32le")
  }
  return bytes.withUnsafeBytes { raw in Array(raw.bindMemory(to: Float.self)) }
}

func model(at path: String) throws -> MLModel {
  let configuration = MLModelConfiguration()
  configuration.computeUnits = .cpuOnly
  return try MLModel(
    contentsOf: URL(fileURLWithPath: path),
    configuration: configuration
  )
}

func frameCount(milliseconds: Int) -> Int {
  max(1, Int(ceil(Double(milliseconds) * 16.0 / Double(frameSamples))))
}

func finishActiveRegion(
  start: Int?,
  end: Int,
  scores: [Float],
  options: RegionOptions,
  into found: inout [Region]
) {
  guard let start,
    let region = completedRegion(
      start: start,
      end: end,
      scores: scores,
      minimumFrames: options.minimumFrames,
      sampleRate: options.sampleRate
    ) else {
    return
  }
  found.append(region)
}

func detectRegions(
  samples: [Float],
  model: MLModel,
  request: Request,
  minSpeechFrames: Int,
  minSilenceFrames: Int
) throws -> [Region] {
  var state = Array(repeating: Float(0), count: stateCount)
  var context = Array(repeating: Float(0), count: contextSamples)
  var activeStart: Int?
  var activeScores: [Float] = []
  var silenceFrames = 0
  var found: [Region] = []
  for start in stride(from: 0, to: samples.count - frameSamples + 1, by: frameSamples) {
    let frame = Array(samples[start..<(start + frameSamples)])
    let probability = try score(model: model, frame: frame, context: &context, state: &state)
    if probability >= request.threshold {
      if activeStart == nil {
        activeStart = start
        activeScores = []
        silenceFrames = 0
      }
      activeScores.append(probability)
      silenceFrames = 0
    } else if activeStart != nil {
      silenceFrames += 1
      if silenceFrames >= minSilenceFrames {
        let end = start - (silenceFrames - 1) * frameSamples
        finishActiveRegion(
          start: activeStart,
          end: end,
          scores: activeScores,
          options: RegionOptions(
            minimumFrames: minSpeechFrames,
            sampleRate: request.sampleRate
          ),
          into: &found
        )
        activeStart = nil
      }
    }
  }
  finishActiveRegion(
    start: activeStart,
    end: samples.count / frameSamples * frameSamples,
    scores: activeScores,
    options: RegionOptions(
      minimumFrames: minSpeechFrames,
      sampleRate: request.sampleRate
    ),
    into: &found
  )
  return found
}

func regions(request: Request) throws -> [Region] {
  guard request.sampleRate == 16_000 else {
    throw WorkerError.invalidRequest("Silero worker only supports 16000 Hz PCM")
  }
  guard request.threshold > 0 && request.threshold < 1 else {
    throw WorkerError.invalidRequest("threshold must be between zero and one")
  }
  return try detectRegions(
    samples: inputSamples(for: request),
    model: model(at: request.modelPath),
    request: request,
    minSpeechFrames: frameCount(milliseconds: request.minSpeechMs),
    minSilenceFrames: frameCount(milliseconds: request.minSilenceMs)
  )
}

do {
  let data = try FileHandle.standardInput.readToEnd() ?? Data()
  let decoder = JSONDecoder()
  decoder.keyDecodingStrategy = .convertFromSnakeCase
  let request = try decoder.decode(Request.self, from: data)
  let response = Response(
    provider: "silero-coreml",
    sampleRate: request.sampleRate,
    regions: try regions(request: request)
  )
  let encoder = JSONEncoder()
  encoder.keyEncodingStrategy = .convertToSnakeCase
  guard let json = String(bytes: try encoder.encode(response), encoding: .utf8) else {
    throw WorkerError.model("could not encode response as UTF-8")
  }
  print(json)
} catch {
  fputs("silero-vad-macos: \(error.localizedDescription)\n", stderr)
  exit(2)
}
