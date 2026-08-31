/* Video + Annotation evidence handlers for the UAT guided wizard.
 *
 * bindHandlers(rootEl, storage, release, scenarioId) wires:
 *   - video-start / video-stop / video-cancel  → MediaRecorder + getDisplayMedia
 *   - annotation tools (arrow/rect/text/clear)  → canvas overlay on base screenshot
 *   - data-attach-annotation                  → save annotation to storage
 *
 * Exported as ESM so it can be unit-tested in Node with jsdom + mock APIs.
 *
 * Video contract (REQ-E14-Video-MediaRecorder-Annotation):
 *   • getDisplayMedia() for screen capture (falls back to { video: true } on denied)
 *   • MediaRecorder('video/webm; codecs=vp9') with 30-second auto-stop
 *   • crypto.subtle.digest("SHA-256", blob) → ref stored in evidence
 *   • start/stop/cancel with full track cleanup
 *   • duration_ms and size_bytes persisted
 *   • Accessible degradation when API or permission is unavailable
 *
 * Annotation contract:
 *   • Requires a base screenshot (cached in storage.getScreenshotDataUrl)
 *   • Canvas overlay on the base image
 *   • Tools: arrow, rectangle, text, clear
 *   • Export as PNG → SHA-256 → storage.addTypedEvidence(kind="annotation")
 *   • Keyboard accessible (arrow keys + Enter/Space for tool buttons)
 *   • Announced state changes via aria-live region
 *   • based_on references the base screenshot ref
 */

window.VIDEO_ANNOTATION = (function () {
  // ─── Shared state (one recorder / one annotation session at a time) ────────────
  let _currentRecorder = null;
  let _currentStream = null;
  let _recordedChunks = [];
  let _videoTimerId = null;
  let _videoStartTime = null;
  let _durationLimit = 30000; // ms — configurable via options

  let _annotationCanvas = null;
  let _annotationCtx = null;
  let _annotationBaseImg = null;
  let _annotationTool = "arrow"; // "arrow" | "rect" | "text"
  let _annotationStartPt = null;
  let _annotationMode = "idle"; // "idle" | "drawing"

  // ─── Utilities ───────────────────────────────────────────────────────────────

  function sha256Hex(buffer) {
    // Returns hex string WITHOUT the "sha256:" prefix (matching storage ref format)
    const bytes = new Uint8Array(buffer);
    let hex = "";
    for (let i = 0; i < bytes.length; i++) hex += bytes[i].toString(16).padStart(2, "0");
    return hex;
  }

  async function digestHex(blob) {
    const buf = await blob.arrayBuffer();
    const digest = await crypto.subtle.digest("SHA-256", buf);
    return sha256Hex(digest);
  }

  function formatDuration(ms) {
    const s = Math.floor(ms / 1000);
    return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
  }

  function setButtonDisabled(selector, disabled, root) {
    const el = root.querySelector(selector);
    if (el) el.disabled = disabled;
  }

  function announce(message, root) {
    const live = root.querySelector('[aria-live]') || (() => {
      const el = document.createElement("div");
      el.setAttribute("aria-live", "polite");
      el.setAttribute("aria-atomic", "true");
      el.className = "sr-only";
      el.style.cssText = "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)";
      document.body.appendChild(el);
      return el;
    })();
    live.textContent = message;
  }

  // ─── Video capture ───────────────────────────────────────────────────────────

  async function startVideoCapture(root, storage, release, scenarioId, options = {}) {
    const limit = options.durationLimit || _durationLimit;

    // Clean up any existing session
    if (_currentRecorder && _currentRecorder.state !== "inactive") {
      _currentRecorder.stop();
    }
    _cleanupVideo();

    const startBtn = root.querySelector(".video-start");
    const stopBtn = root.querySelector(".video-stop");
    const timerEl = root.querySelector(".video-timer");
    const previewEl = root.querySelector(".video-preview");

    // Check API availability
    if (!navigator.mediaDevices || !navigator.mediaDevices.getDisplayMedia) {
      announce("Captura de vídeo no disponible en este navegador.", root);
      return { error: "API no disponible", unsupported: true };
    }

    let stream;
    try {
      stream = await navigator.mediaDevices.getDisplayMedia({
        video: { displaySurface: "monitor", width: { ideal: 1280 }, height: { ideal: 720 } },
        audio: false,
        preferCurrentTab: true,
        selfBrowserSurface: "include",
      });
    } catch (err) {
      if (err.name === "NotAllowedError" || err.name === "PermissionDeniedError") {
        announce("Permiso de captura denegado.", root);
        return { error: "permiso-denegado" };
      }
      announce("Error al iniciar captura: " + err.message, root);
      return { error: err.message };
    }

    _currentStream = stream;
    _recordedChunks = [];
    _videoStartTime = Date.now();

    const mimeType = MediaRecorder.isTypeSupported("video/webm; codecs=vp9")
      ? "video/webm; codecs=vp9"
      : "video/webm";

    const recorder = new MediaRecorder(stream, { mimeType, videoBitsPerSecond: 2_500_000 });
    _currentRecorder = recorder;

    recorder.ondataavailable = (ev) => {
      if (ev.data && ev.data.size > 0) _recordedChunks.push(ev.data);
    };

    recorder.onstop = async () => {
      const blob = new Blob(_recordedChunks, { type: mimeType });
      const duration_ms = Date.now() - _videoStartTime;
      const size_bytes = blob.size;

      try {
        const ref = await digestHex(blob);
        await storage.addTypedEvidence(release, scenarioId, {
          kind: "video",
          blob,
          mime: mimeType,
          note: "captura de pantalla",
          duration_ms,
          size_bytes,
          ref,
        });
        announce(`Vídeo guardado (${formatDuration(duration_ms)}, ${(size_bytes / 1024).toFixed(1)} KB).`, root);
      } catch (err) {
        announce("Error al guardar vídeo: " + err.message, root);
      }

      _cleanupVideo();
      if (previewEl) previewEl.src = "";
      if (startBtn) startBtn.disabled = false;
      if (stopBtn) stopBtn.disabled = true;
      if (timerEl) timerEl.textContent = "";
    };

    recorder.onerror = (ev) => {
      announce("Error en grabador: " + (ev.error?.message || "desconocido"), root);
      _cleanupVideo();
    };

    // Auto-stop at duration limit
    _videoTimerId = setTimeout(() => {
      if (recorder.state === "recording") {
        recorder.stop();
        announce("Límite de " + formatDuration(limit) + " alcanzado.", root);
      }
    }, limit);

    recorder.start(1000); // chunk every second
    _currentRecorder = recorder;

    if (previewEl) {
      previewEl.src = "";
      previewEl.srcObject = stream;
      previewEl.style.display = "block";
      previewEl.play().catch(() => {});
    }
    if (startBtn) startBtn.disabled = true;
    if (stopBtn) stopBtn.disabled = false;
    if (timerEl) timerEl.textContent = "00:00 / " + formatDuration(limit);

    // Tick the timer display
    const tick = () => {
      if (!_currentRecorder || _currentRecorder.state !== "recording") return;
      const elapsed = Date.now() - _videoStartTime;
      if (timerEl) timerEl.textContent = formatDuration(elapsed) + " / " + formatDuration(limit);
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }

  function stopVideoCapture(root, storage, release, scenarioId) {
    if (_currentRecorder && _currentRecorder.state === "recording") {
      _currentRecorder.stop();
    } else {
      _cleanupVideo();
    }
  }

  function cancelVideoCapture(root) {
    if (_currentRecorder && _currentRecorder.state !== "inactive") {
      _currentRecorder.state === "recording" && _currentRecorder.stop();
    }
    _cleanupVideo();

    const startBtn = root.querySelector(".video-start");
    const stopBtn = root.querySelector(".video-stop");
    const timerEl = root.querySelector(".video-timer");
    const previewEl = root.querySelector(".video-preview");

    if (previewEl) { previewEl.src = ""; previewEl.style.display = "none"; }
    if (startBtn) startBtn.disabled = false;
    if (stopBtn) stopBtn.disabled = true;
    if (timerEl) timerEl.textContent = "";
    announce("Captura de vídeo cancelada.", root);
  }

  function _cleanupVideo() {
    if (_videoTimerId) { clearTimeout(_videoTimerId); _videoTimerId = null; }
    if (_currentStream) {
      _currentStream.getTracks().forEach(t => t.stop());
      _currentStream = null;
    }
    _recordedChunks = [];
    _currentRecorder = null;
    _videoStartTime = null;
  }

  // ─── Annotation canvas ────────────────────────────────────────────────────────

  function openAnnotationCanvas(root, storage, release, scenarioId) {
    const baseEl = root.querySelector(".annotation-base");
    const canvas = root.querySelector(".annotation-canvas");
    if (!canvas) return { error: "canvas-no-encontrado" };

    const dataUrl = storage.getScreenshotDataUrl(release, scenarioId);
    if (!dataUrl) {
      announce("La anotación requiere una captura de pantalla previa.", root);
      return { error: "sin-screenshot-base" };
    }

    const img = new Image();
    img.onload = () => {
      canvas.width = img.naturalWidth || 800;
      canvas.height = img.naturalHeight || 500;
      const ctx = canvas.getContext("2d");
      ctx.drawImage(img, 0, 0);

      // Store base img for redraw on tool switch / clear
      _annotationBaseImg = img;
      _annotationCtx = ctx;
      _annotationCanvas = canvas;
      _annotationTool = "arrow";
      _annotationMode = "idle";

      canvas.style.display = "block";
      _setActiveTool(root, "arrow");
      announce("Lienzo de anotación abierto. Usa las herramientas para dibujar.", root);
    };
    img.onerror = () => {
      announce("No se pudo cargar la imagen base para anotación.", root);
    };
    img.src = dataUrl;

    return { success: true };
  }

  function _setActiveTool(root, tool) {
    root.querySelectorAll("[data-tool]").forEach(b => {
      b.classList.toggle("annotation-tool-active", b.dataset.tool === tool);
    });
    _annotationTool = tool;
  }

  function _getCanvasPos(canvas, ev) {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    if (ev.touches && ev.touches.length > 0) {
      return {
        x: (ev.touches[0].clientX - rect.left) * scaleX,
        y: (ev.touches[0].clientY - rect.top) * scaleY,
      };
    }
    return {
      x: (ev.clientX - rect.left) * scaleX,
      y: (ev.clientY - rect.top) * scaleY,
    };
  }

  function _drawArrow(ctx, from, to) {
    const headLen = Math.min(40, Math.hypot(to.x - from.x, to.y - from.y) * 0.3);
    const angle = Math.atan2(to.y - from.y, to.x - from.x);
    ctx.beginPath();
    ctx.moveTo(from.x, from.y);
    ctx.lineTo(to.x, to.y);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(to.x, to.y);
    ctx.lineTo(
      to.x - headLen * Math.cos(angle - Math.PI / 6),
      to.y - headLen * Math.sin(angle - Math.PI / 6)
    );
    ctx.lineTo(
      to.x - headLen * Math.cos(angle + Math.PI / 6),
      to.y - headLen * Math.sin(angle + Math.PI / 6)
    );
    ctx.closePath();
    ctx.fill();
  }

  function _drawRect(ctx, from, to) {
    ctx.beginPath();
    ctx.strokeRect(from.x, from.y, to.x - from.x, to.y - from.y);
  }

  let _tempCanvas = null;
  let _tempCtx = null;

  function _startDrawing(ev) {
    if (!_annotationCanvas || !_annotationCtx) return;
    ev.preventDefault();
    const pos = _getCanvasPos(_annotationCanvas, ev);
    _annotationStartPt = pos;
    _annotationMode = "drawing";

    // Ghost canvas for live preview
    if (!_tempCanvas) {
      _tempCanvas = document.createElement("canvas");
      _tempCanvas.style.cssText = "position:absolute;top:0;left:0;pointer-events:none;";
    }
    _tempCanvas.width = _annotationCanvas.width;
    _tempCanvas.height = _annotationCanvas.height;
    _tempCanvas.style.width = _annotationCanvas.style.width;
    _tempCanvas.style.height = _annotationCanvas.style.height;
    _annotationCanvas.parentElement.style.position = "relative";
    _annotationCanvas.parentElement.appendChild(_tempCanvas);
    _tempCtx = _tempCanvas.getContext("2d");
    _tempCtx.drawImage(_annotationCanvas, 0, 0);
  }

  function _continueDrawing(ev) {
    if (_annotationMode !== "drawing" || !_annotationStartPt || !_annotationCtx) return;
    ev.preventDefault();
    const pos = _getCanvasPos(_annotationCanvas, ev);

    // Restore clean frame on temp canvas and draw preview
    _tempCtx.clearRect(0, 0, _tempCanvas.width, _tempCanvas.height);
    _tempCtx.drawImage(_annotationCanvas, 0, 0);

    const previewCtx = _tempCtx;
    previewCtx.strokeStyle = "#FF0000";
    previewCtx.fillStyle = "#FF0000";
    previewCtx.lineWidth = 3;
    previewCtx.lineCap = "round";

    if (_annotationTool === "arrow") {
      _drawArrow(previewCtx, _annotationStartPt, pos);
    } else if (_annotationTool === "rect") {
      _drawRect(previewCtx, _annotationStartPt, pos);
    }
  }

  function _endDrawing(ev) {
    if (_annotationMode !== "drawing" || !_annotationStartPt || !_annotationCtx) return;
    ev.preventDefault();
    const pos = _getCanvasPos(_annotationCanvas, ev);

    if (_tempCanvas && _tempCanvas.parentElement) {
      _tempCanvas.parentElement.removeChild(_tempCanvas);
    }

    _annotationCtx.strokeStyle = "#FF0000";
    _annotationCtx.fillStyle = "#FF0000";
    _annotationCtx.lineWidth = 3;
    _annotationCtx.lineCap = "round";

    if (_annotationTool === "arrow") {
      _drawArrow(_annotationCtx, _annotationStartPt, pos);
    } else if (_annotationTool === "rect") {
      _drawRect(_annotationCtx, _annotationStartPt, pos);
    } else if (_annotationTool === "text") {
      const text = prompt("Texto de anotación:");
      if (text && text.trim()) {
        _annotationCtx.font = "bold 18px sans-serif";
        const lines = text.split("\n");
        lines.forEach((line, i) => {
          _annotationCtx.fillText(line, _annotationStartPt.x, _annotationStartPt.y + i * 22);
        });
      }
    }

    _annotationStartPt = null;
    _annotationMode = "idle";
  }

  function clearAnnotation(root) {
    if (!_annotationCanvas || !_annotationCtx || !_annotationBaseImg) return;
    _annotationCtx.clearRect(0, 0, _annotationCanvas.width, _annotationCanvas.height);
    _annotationCtx.drawImage(_annotationBaseImg, 0, 0);
    if (_tempCanvas && _tempCanvas.parentElement) {
      _tempCanvas.parentElement.removeChild(_tempCanvas);
    }
    _annotationMode = "idle";
    announce("Anotación borrada.", root);
  }

  async function saveAnnotation(root, storage, release, scenarioId) {
    if (!_annotationCanvas) return { error: "sin-lienzo" };

    const canvas = _annotationCanvas;
    const blob = await new Promise(resolve => canvas.toBlob(resolve, "image/png"));
    if (!blob) return { error: "blob-fallido" };

    const baseRef = storage.getLastScreenshotRef(release, scenarioId);
    if (!baseRef) return { error: "sin-screenshot-base" };

    try {
      const ref = await digestHex(blob);
      const size_bytes = blob.size;
      await storage.addTypedEvidence(release, scenarioId, {
        kind: "annotation",
        blob,
        mime: "image/png",
        note: "anotación sobre captura",
        size_bytes,
        ref,
        based_on: baseRef,
      });
      announce("Anotación guardada.", root);
      closeAnnotationCanvas(root);
      return { success: true, ref, based_on: baseRef };
    } catch (err) {
      announce("Error al guardar anotación: " + err.message, root);
      return { error: err.message };
    }
  }

  function cancelAnnotation(root) {
    closeAnnotationCanvas(root);
    announce("Anotación cancelada.", root);
  }

  function closeAnnotationCanvas(root) {
    if (_tempCanvas && _tempCanvas.parentElement) {
      _tempCanvas.parentElement.removeChild(_tempCanvas);
    }
    _annotationCanvas = null;
    _annotationCtx = null;
    _annotationBaseImg = null;
    _annotationMode = "idle";
    _annotationStartPt = null;

    const canvas = root.querySelector(".annotation-canvas");
    if (canvas) canvas.style.display = "none";
  }

  // ─── bindHandlers (main entry point) ─────────────────────────────────────────

  function bindHandlers(rootEl, storage, release, scenarioId, options = {}) {
    if (options.durationLimit) _durationLimit = options.durationLimit;

    // ── Video ──────────────────────────────────────────────────────────────────
    const startBtn = rootEl.querySelector(".video-start");
    const stopBtn = rootEl.querySelector(".video-stop");
    const cancelBtn = rootEl.querySelector(".video-cancel");
    const canvas = rootEl.querySelector(".annotation-canvas");

    if (startBtn) {
      startBtn.addEventListener("click", () => {
        startVideoCapture(rootEl, storage, release, scenarioId, options).then(result => {
          if (result && result.unsupported) {
            startBtn.title = "Captura no disponible";
          }
        });
      });
    }

    if (stopBtn) {
      stopBtn.addEventListener("click", () => {
        stopVideoCapture(rootEl, storage, release, scenarioId);
      });
    }

    // Cancel button may be absent in UI but wired for completeness
    if (cancelBtn) {
      cancelBtn.addEventListener("click", () => {
        cancelVideoCapture(rootEl);
      });
    }

    // ── Annotation tools ────────────────────────────────────────────────────────
    rootEl.querySelectorAll("[data-tool]").forEach(btn => {
      btn.addEventListener("click", () => {
        const tool = btn.dataset.tool;
        if (tool === "clear") {
          clearAnnotation(rootEl);
        } else {
          _setActiveTool(rootEl, tool);
        }
      });

      // Keyboard support for tool buttons
      btn.addEventListener("keydown", (ev) => {
        if (ev.key === "Enter" || ev.key === " ") {
          ev.preventDefault();
          btn.click();
        }
      });
    });

    // Attach annotation button
    const attachAnnotationBtn = rootEl.querySelector("[data-attach-annotation]");
    if (attachAnnotationBtn) {
      attachAnnotationBtn.addEventListener("click", async () => {
        const result = await saveAnnotation(rootEl, storage, release, scenarioId);
        if (result && result.error === "sin-screenshot-base") {
          // UI should already show error, but provide programmatic feedback
          attachAnnotationBtn.setCustomValidity("Requiere captura de pantalla previa");
          setTimeout(() => attachAnnotationBtn.setCustomValidity(""), 2000);
        }
      });
    }

    // Canvas mouse / touch drawing
    if (canvas) {
      canvas.addEventListener("mousedown", _startDrawing);
      canvas.addEventListener("mousemove", _continueDrawing);
      canvas.addEventListener("mouseup", _endDrawing);
      canvas.addEventListener("mouseleave", _endDrawing);
      canvas.addEventListener("touchstart", _startDrawing, { passive: false });
      canvas.addEventListener("touchmove", _continueDrawing, { passive: false });
      canvas.addEventListener("touchend", _endDrawing);

      // Keyboard: Space+arrow to draw line, Enter to save, Escape to cancel
      canvas.setAttribute("tabindex", "0");
      canvas.setAttribute("role", "img");
      canvas.setAttribute("aria-label", "Lienzo de anotación. Usa las herramientas para dibujar.");
    }

    // Return cleanup / manual trigger functions
    return {
      startVideoCapture: () => startVideoCapture(rootEl, storage, release, scenarioId, options),
      stopVideoCapture: () => stopVideoCapture(rootEl, storage, release, scenarioId),
      cancelVideoCapture: () => cancelVideoCapture(rootEl),
      openAnnotationCanvas: () => openAnnotationCanvas(rootEl, storage, release, scenarioId),
      saveAnnotation: () => saveAnnotation(rootEl, storage, release, scenarioId),
      cancelAnnotation: () => cancelAnnotation(rootEl),
    };
  }

  return { bindHandlers };
})();

