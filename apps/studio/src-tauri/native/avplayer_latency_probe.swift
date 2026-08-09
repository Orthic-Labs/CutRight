import AVFoundation
import AppKit
import Foundation

// Standalone AVPlayer latency probe for CR-F-B8-003. Not part of the app build;
// run manually via `swift avplayer_latency_probe.swift <path> [iterations]`.
//
// Displayability signal: AVPlayerLayer.isReadyForDisplay (KVO). Per Apple docs
// this becomes true only once the layer has an actual frame ready to show on
// screen, which is stronger than AVPlayerItem.status == .readyToPlay (that
// only means metadata/tracks are loaded, not that a frame has been decoded).

func nowMs() -> Double { Double(DispatchTime.now().uptimeNanoseconds) / 1_000_000.0 }

func percentile(_ sorted: [Double], _ p: Double) -> Double {
  guard !sorted.isEmpty else { return 0 }
  let idx = min(sorted.count - 1, max(0, Int((p * Double(sorted.count - 1)).rounded())))
  return sorted[idx]
}

func stats(_ samples: [Double]) -> [String: Double] {
  let sorted = samples.sorted()
  return [
    "p50": percentile(sorted, 0.50),
    "p95": percentile(sorted, 0.95),
    "min": sorted.first ?? 0,
    "max": sorted.last ?? 0,
  ]
}

final class ReadyForDisplayWaiter: NSObject {
  private var observation: NSKeyValueObservation?
  private var completion: (() -> Void)?

  func wait(layer: AVPlayerLayer, timeout: TimeInterval, completion: @escaping () -> Void) {
    if layer.isReadyForDisplay {
      completion()
      return
    }
    self.completion = completion
    observation = layer.observe(\.isReadyForDisplay, options: [.new]) { [weak self] _, change in
      guard change.newValue == true else { return }
      self?.fire()
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + timeout) { [weak self] in self?.fire() }
  }

  private func fire() {
    observation?.invalidate()
    observation = nil
    let cb = completion
    completion = nil
    cb?()
  }
}

func run() {
  let args = CommandLine.arguments
  guard args.count >= 2 else {
    FileHandle.standardError.write("usage: avplayer_latency_probe.swift <media-path> [iterations]\n".data(using: .utf8)!)
    exit(2)
  }
  let mediaPath = args[1]
  let iterations = args.count >= 3 ? (Int(args[2]) ?? 20) : 20
  guard FileManager.default.fileExists(atPath: mediaPath) else {
    FileHandle.standardError.write("media file not found: \(mediaPath)\n".data(using: .utf8)!)
    exit(2)
  }
  let url = URL(fileURLWithPath: mediaPath)
  let asset = AVURLAsset(url: url)
  // `asset.duration` is deprecated on macOS 13+; load it asynchronously and
  // block, since the probe's main flow is synchronous.
  var loadedDuration = CMTime.zero
  let durationLoaded = DispatchSemaphore(value: 0)
  Task {
    loadedDuration = (try? await asset.load(.duration)) ?? .zero
    durationLoaded.signal()
  }
  durationLoaded.wait()
  let durationSeconds = CMTimeGetSeconds(loadedDuration)

  let player = AVPlayer()
  let layer = AVPlayerLayer(player: player)
  layer.videoGravity = .resizeAspect
  let hostView = NSView(frame: NSRect(x: 0, y: 0, width: 640, height: 360))
  hostView.wantsLayer = true
  hostView.layer?.addSublayer(layer)
  layer.frame = hostView.bounds

  var loadSamples: [Double] = []
  var seekSamples: [Double] = []
  let waiter = ReadyForDisplayWaiter()

  func measureLoad(_ done: @escaping () -> Void) {
    let item = AVPlayerItem(url: url)
    let t0 = nowMs()
    var itemReady = false
    var frameReady = false
    var statusObs: NSKeyValueObservation?
    func maybeFinish() {
      guard itemReady, frameReady else { return }
      statusObs?.invalidate()
      loadSamples.append(nowMs() - t0)
      done()
    }
    statusObs = item.observe(\.status, options: [.new]) { obj, _ in
      guard obj.status == .readyToPlay else { return }
      itemReady = true
      maybeFinish()
    }
    player.replaceCurrentItem(with: item)
    waiter.wait(layer: layer, timeout: 10) {
      frameReady = true
      maybeFinish()
    }
  }

  func measureSeek(offsetSeconds: Double, _ done: @escaping () -> Void) {
    let target = CMTime(seconds: offsetSeconds, preferredTimescale: 600)
    let t0 = nowMs()
    player.seek(to: target, toleranceBefore: .zero, toleranceAfter: .zero) { _ in
      waiter.wait(layer: layer, timeout: 10) {
        seekSamples.append(nowMs() - t0)
        done()
      }
    }
  }

  // Chain iterations serially on the main run loop.
  var i = 0
  func nextLoad() {
    if i >= iterations {
      i = 0
      nextSeek()
      return
    }
    i += 1
    measureLoad { DispatchQueue.main.async { nextLoad() } }
  }
  func nextSeek() {
    if i >= iterations {
      finish()
      return
    }
    let frac = Double(i) / Double(max(1, iterations - 1))
    let offset = max(0.0, min(durationSeconds - 0.1, frac * durationSeconds))
    i += 1
    measureSeek(offsetSeconds: offset) { DispatchQueue.main.async { nextSeek() } }
  }
  func finish() {
    let loadStats = stats(loadSamples)
    let seekStats = stats(seekSamples)
    var hw = utsname()
    uname(&hw)
    let machine = withUnsafePointer(to: &hw.machine) { p in
      p.withMemoryRebound(to: CChar.self, capacity: 1) { String(cString: $0) }
    }
    let osVersion = ProcessInfo.processInfo.operatingSystemVersionString
    let payload: [String: Any] = [
      "signal": "AVPlayerLayer.isReadyForDisplay",
      "mediaPath": mediaPath,
      "mediaDurationSeconds": durationSeconds,
      "iterations": iterations,
      "macOSVersion": osVersion,
      "hardware": machine,
      "loadToFirstFrameMs": [
        "p50": loadStats["p50"]!, "p95": loadStats["p95"]!,
        "min": loadStats["min"]!, "max": loadStats["max"]!,
        "samples": loadSamples,
      ],
      "seekToFrameMs": [
        "p50": seekStats["p50"]!, "p95": seekStats["p95"]!,
        "min": seekStats["min"]!, "max": seekStats["max"]!,
        "samples": seekSamples,
      ],
    ]
    let data = try! JSONSerialization.data(withJSONObject: payload, options: [.prettyPrinted, .sortedKeys])
    print(String(data: data, encoding: .utf8)!)
    exit(0)
  }
  nextLoad()
  RunLoop.main.run()
}

run()
