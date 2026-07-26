import AVFoundation
import Foundation
import QuartzCore

struct Request: Decodable {
  let inputPath: String
  let captionsPath: String
  let outputPath: String
}

struct Cue {
  let start: Double
  let end: Double
  let text: String
}

func timestamp(_ value: Substring) -> Double? {
  let parts = value.replacingOccurrences(of: ",", with: ".").split(separator: ":")
  guard parts.count == 3,
    let hours = Double(parts[0]),
    let minutes = Double(parts[1]),
    let seconds = Double(parts[2]) else {
    return nil
  }
  return hours * 3600 + minutes * 60 + seconds
}

func readSrt(_ path: String) throws -> [Cue] {
  let source = try String(contentsOfFile: path, encoding: .utf8)
  return source.components(separatedBy: "\n\n").compactMap { chunk in
    let lines = chunk.split(whereSeparator: \.isNewline)
    guard lines.count >= 3, let arrow = lines[1].range(of: " --> ") else { return nil }
    let start = timestamp(lines[1][..<arrow.lowerBound])
    let end = timestamp(lines[1][arrow.upperBound...])
    guard let start, let end, end > start else { return nil }
    return Cue(start: start, end: end, text: lines.dropFirst(2).joined(separator: "\n"))
  }
}

func cutRightError(_ code: Int, _ message: String) -> NSError {
  NSError(domain: "CutRight", code: code, userInfo: [NSLocalizedDescriptionKey: message])
}

func captionLayer(for cue: Cue, in renderSize: CGSize) -> CATextLayer {
  let layer = CATextLayer()
  layer.frame = CGRect(
    x: renderSize.width * 0.08,
    y: renderSize.height * 0.10,
    width: renderSize.width * 0.84,
    height: renderSize.height * 0.20
  )
  layer.string = cue.text
  layer.fontSize = max(28, renderSize.height * 0.045)
  layer.alignmentMode = .center
  layer.foregroundColor = CGColor(gray: 1, alpha: 1)
  layer.backgroundColor = CGColor(gray: 0, alpha: 0.70)
  layer.cornerRadius = 10
  layer.isWrapped = true
  layer.opacity = 0
  let show = CABasicAnimation(keyPath: "opacity")
  show.fromValue = 1
  show.toValue = 1
  show.beginTime = AVCoreAnimationBeginTimeAtZero + cue.start
  show.duration = max(0.05, cue.end - cue.start)
  show.fillMode = .both
  show.isRemovedOnCompletion = false
  layer.add(show, forKey: "cue-\(cue.start)")
  return layer
}

func copyTracks(
  from asset: AVURLAsset,
  video: AVAssetTrack
) throws -> (AVMutableComposition, AVMutableCompositionTrack) {
  let composition = AVMutableComposition()
  guard let videoTrack = composition.addMutableTrack(
    withMediaType: .video,
    preferredTrackID: kCMPersistentTrackID_Invalid
  ) else {
    throw cutRightError(2, "cannot create video composition track")
  }
  let range = CMTimeRange(start: .zero, duration: asset.duration)
  try videoTrack.insertTimeRange(range, of: video, at: .zero)
  if let audio = asset.tracks(withMediaType: .audio).first,
    let audioTrack = composition.addMutableTrack(
      withMediaType: .audio,
      preferredTrackID: kCMPersistentTrackID_Invalid
    ) {
    try audioTrack.insertTimeRange(range, of: audio, at: .zero)
  }
  return (composition, videoTrack)
}

func captionComposition(
  video: AVAssetTrack,
  videoTrack: AVCompositionTrack,
  duration: CMTime,
  cues: [Cue]
) -> AVMutableVideoComposition {
  let transformedSize = video.naturalSize.applying(video.preferredTransform)
  let renderSize = CGSize(width: abs(transformedSize.width), height: abs(transformedSize.height))
  let instruction = AVMutableVideoCompositionInstruction()
  instruction.timeRange = CMTimeRange(start: .zero, duration: duration)
  let layerInstruction = AVMutableVideoCompositionLayerInstruction(assetTrack: videoTrack)
  layerInstruction.setTransform(video.preferredTransform, at: .zero)
  instruction.layerInstructions = [layerInstruction]
  let videoComposition = AVMutableVideoComposition()
  videoComposition.renderSize = renderSize
  videoComposition.frameDuration = CMTime(value: 1, timescale: 30)
  videoComposition.instructions = [instruction]

  let parent = CALayer()
  parent.frame = CGRect(origin: .zero, size: renderSize)
  let videoLayer = CALayer()
  videoLayer.frame = parent.frame
  parent.addSublayer(videoLayer)
  for cue in cues {
    parent.addSublayer(captionLayer(for: cue, in: renderSize))
  }
  videoComposition.animationTool = AVVideoCompositionCoreAnimationTool(
    postProcessingAsVideoLayer: videoLayer,
    in: parent
  )
  return videoComposition
}

func export(
  composition: AVMutableComposition,
  videoComposition: AVMutableVideoComposition,
  outputPath: String
) throws {
  let output = URL(fileURLWithPath: outputPath)
  try? FileManager.default.removeItem(at: output)
  guard let exporter = AVAssetExportSession(
    asset: composition,
    presetName: AVAssetExportPresetHighestQuality
  ) else {
    throw cutRightError(3, "cannot create exporter")
  }
  exporter.outputURL = output
  exporter.outputFileType = .mp4
  exporter.videoComposition = videoComposition
  let semaphore = DispatchSemaphore(value: 0)
  exporter.exportAsynchronously { semaphore.signal() }
  semaphore.wait()
  guard exporter.status == .completed else {
    throw exporter.error ?? cutRightError(4, "caption export failed")
  }
}

func render(_ request: Request) throws {
  let asset = AVURLAsset(url: URL(fileURLWithPath: request.inputPath))
  guard let video = asset.tracks(withMediaType: .video).first else {
    throw cutRightError(1, "input has no video track")
  }
  let (composition, videoTrack) = try copyTracks(from: asset, video: video)
  let videoComposition = captionComposition(
    video: video,
    videoTrack: videoTrack,
    duration: asset.duration,
    cues: try readSrt(request.captionsPath)
  )
  try export(
    composition: composition,
    videoComposition: videoComposition,
    outputPath: request.outputPath
  )
}

do {
  let data = try FileHandle.standardInput.readToEnd() ?? Data()
  let decoder = JSONDecoder()
  decoder.keyDecodingStrategy = .convertFromSnakeCase
  let request = try decoder.decode(Request.self, from: data)
  try render(request)
} catch {
  fputs("captions-macos: \(error.localizedDescription)\n", stderr)
  exit(2)
}
