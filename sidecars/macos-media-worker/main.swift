import Foundation

private let encoder = JSONEncoder()
private let decoder = JSONDecoder()
private var liveRequestIds = Set<String>()
private var acceptingRequests = true
private let serviceQueue = DispatchQueue(label: "com.cutright.macos-media.service")
private let outputLock = NSLock()
private let requestLock = NSLock()
private let assetService = AssetService()
private let visionService = VisionService()
private let renderService = RenderService()
private let audioService = AudioService()
private let timelineRenderService = TimelineRenderService()

private func capabilities() -> MacMediaCapabilities {
    MacMediaCapabilities(avFoundation: true, vision: true, caption: true, preview: renderService.supportsPreview, audio: true, metal: renderService.supportsPreview, osVersion: ProcessInfo.processInfo.operatingSystemVersionString, workerVersion: "1")
}

private func decodePayload<T: Decodable>(_ payload: JSONValue, as type: T.Type) throws -> T {
    try decoder.decode(T.self, from: encoder.encode(payload))
}

private func jsonValue<T: Encodable>(_ value: T) throws -> JSONValue {
    try decoder.decode(JSONValue.self, from: encoder.encode(value))
}

private func serviceFailure(_ request: RequestEnvelope, _ error: Error) -> ResponseEnvelope {
    if let error = error as? MacMediaServiceError { return failure(request, error.code, String(describing: error)) }
    return failure(request, "service_failed", error.localizedDescription)
}

private func reserve(_ request: RequestEnvelope) -> Bool {
    requestLock.lock()
    defer { requestLock.unlock() }
    guard !liveRequestIds.contains(request.requestId) else { return false }
    liveRequestIds.insert(request.requestId)
    return true
}

private func dispatch(_ request: RequestEnvelope) -> ResponseEnvelope {
    let started = DispatchTime.now().uptimeNanoseconds
    defer { requestLock.lock(); liveRequestIds.remove(request.requestId); requestLock.unlock() }
    guard request.protocolVersion == macMediaProtocolVersion else {
        return failure(request, "unsupportedVersion", "protocol version \(request.protocolVersion) is unsupported")
    }
    let result: ResponseEnvelope
    switch request.operation {
    case "hello":
        result = ResponseEnvelope(protocolVersion: macMediaProtocolVersion, requestId: request.requestId, operation: request.operation, ok: true, result: .object(["protocolVersion": .number(Double(macMediaProtocolVersion))]), error: nil, capabilities: capabilities(), elapsedNanoseconds: 0)
    case "cancel":
        result = failure(request, "unsupported", "cancellation is owned by Rust worker supervision")
    case "shutdown":
        acceptingRequests = false
        result = ResponseEnvelope(protocolVersion: macMediaProtocolVersion, requestId: request.requestId, operation: request.operation, ok: true, result: .object(["shuttingDown": .bool(true)]), error: nil, capabilities: nil, elapsedNanoseconds: 0)
    case "inspectAsset":
        do { let payload = try decodePayload(request.payload, as: MacAssetInspectRequest.self); result = ResponseEnvelope(protocolVersion: macMediaProtocolVersion, requestId: request.requestId, operation: request.operation, ok: true, result: try jsonValue(assetService.inspectAsset(sourcePath: payload.sourcePath, allowedRoots: payload.allowedRoots)), error: nil, capabilities: nil, elapsedNanoseconds: 0) } catch { result = serviceFailure(request, error) }
    case "analyzeFrames":
        do { let payload = try decodePayload(request.payload, as: MacVisionBatchRequest.self); result = ResponseEnvelope(protocolVersion: macMediaProtocolVersion, requestId: request.requestId, operation: request.operation, ok: true, result: try jsonValue(visionService.analyzeFrames(payload.frames, allowedRoots: payload.allowedRoots)), error: nil, capabilities: nil, elapsedNanoseconds: 0) } catch { result = serviceFailure(request, error) }
    case "renderCaption":
        do { let payload = try decodePayload(request.payload, as: MacCaptionRenderRequest.self); result = ResponseEnvelope(protocolVersion: macMediaProtocolVersion, requestId: request.requestId, operation: request.operation, ok: true, result: try jsonValue(renderService.renderCaption(payload)), error: nil, capabilities: nil, elapsedNanoseconds: 0) } catch { result = serviceFailure(request, error) }
    case "renderPreview":
        do { let payload = try decodePayload(request.payload, as: MacPreviewRenderRequest.self); result = ResponseEnvelope(protocolVersion: macMediaProtocolVersion, requestId: request.requestId, operation: request.operation, ok: true, result: try jsonValue(renderService.renderPreview(payload)), error: nil, capabilities: nil, elapsedNanoseconds: 0) } catch { result = serviceFailure(request, error) }
    case "audioFeatures":
        do { let payload = try decodePayload(request.payload, as: MacAudioFeaturesRequest.self); result = ResponseEnvelope(protocolVersion: macMediaProtocolVersion, requestId: request.requestId, operation: request.operation, ok: true, result: try jsonValue(audioService.audioFeatures(payload)), error: nil, capabilities: nil, elapsedNanoseconds: 0) } catch { result = serviceFailure(request, error) }
    case "renderTimeline":
        do { let payload = try decodePayload(request.payload, as: TimelineRenderRequest.self); result = ResponseEnvelope(protocolVersion: macMediaProtocolVersion, requestId: request.requestId, operation: request.operation, ok: true, result: try jsonValue(timelineRenderService.render(payload)), error: nil, capabilities: nil, elapsedNanoseconds: 0) } catch { result = serviceFailure(request, error) }
    default:
        result = failure(request, "unknownOperation", "unknown operation \(request.operation)")
    }
    let elapsed = DispatchTime.now().uptimeNanoseconds - started
    return ResponseEnvelope(protocolVersion: result.protocolVersion, requestId: result.requestId, operation: result.operation, ok: result.ok, result: result.result, error: result.error, capabilities: result.capabilities, elapsedNanoseconds: elapsed)
}

private func emit(_ response: ResponseEnvelope) {
    guard let data = try? encoder.encode(response),
          let text = String(data: data, encoding: .utf8) else { return }
    outputLock.lock()
    print(text)
    fflush(stdout)
    outputLock.unlock()
}

private func accept(_ request: RequestEnvelope) {
    guard reserve(request) else {
        emit(failure(request, "duplicateRequestId", "request ID is already live"))
        return
    }
    if request.operation == "cancel" {
        emit(dispatch(request))
    } else if request.operation == "shutdown" {
        emit(serviceQueue.sync { dispatch(request) })
    } else {
        serviceQueue.async { emit(dispatch(request)) }
    }
}

private func acceptLine(_ bytes: [UInt8]) {
    var bytes = bytes
    if bytes.last == 13 { bytes.removeLast() }
    guard let request = try? decoder.decode(RequestEnvelope.self, from: Data(bytes)) else { return }
    accept(request)
}

var pendingLine = [UInt8]()
pendingLine.reserveCapacity(4096)
var discardingOversizedLine = false
while acceptingRequests {
    let chunk = FileHandle.standardInput.availableData
    if chunk.isEmpty { break }
    for byte in chunk {
        if !acceptingRequests { break }
        if byte == 10 {
            if !discardingOversizedLine { acceptLine(pendingLine) }
            pendingLine.removeAll(keepingCapacity: true)
            discardingOversizedLine = false
        } else if !discardingOversizedLine {
            if pendingLine.count < maxJsonlLineBytes {
                pendingLine.append(byte)
            } else {
                pendingLine.removeAll(keepingCapacity: true)
                discardingOversizedLine = true
            }
        }
    }
}
if !discardingOversizedLine, !pendingLine.isEmpty { acceptLine(pendingLine) }
serviceQueue.sync {}
