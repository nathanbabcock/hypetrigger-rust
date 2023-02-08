import { createEffect, createSignal, onCleanup } from 'solid-js'
import { Hypetrigger, initWasm } from '../../../lib-js/src'
import {
  initTesseractScheduler,
  TesseractTrigger,
} from './../../../lib-js/src/tesseract'

const greetings = [
  'hey',
  'hi',
  'hello',
  'yo',
  'sup',
  'howdy',
  'hola',
  'bonjour',
]

export default function App() {
  let canvas: HTMLCanvasElement | undefined
  let ctx: CanvasRenderingContext2D | undefined
  let hypetrigger: Hypetrigger | undefined

  const [mousedown, setMousedown] = createSignal(false)
  const [mousePos, setMousePos] = createSignal<
    { x: number; y: number } | undefined
  >()
  const [penSize, _setPenSize] = createSignal(5)
  const [dirty, setDirty] = createSignal(false)

  const [recognizedText, setRecognizedText] = createSignal<
    { text: string; timeMS: number } | undefined
  >()
  const responseText = () =>
    greetings.some(
      greeting =>
        recognizedText()?.text.toLowerCase().includes(greeting.toLowerCase()) ??
        false
    )
      ? 'Hello to you too!'
      : undefined

  createEffect(() =>
    console.log({
      yourText: recognizedText()?.text,
      responseText: responseText(),
    })
  )

  const startDrawing = (e: Event) => {
    setMousedown(true)
    e.preventDefault()
  }

  const stopDrawing = (e: Event) => {
    setMousedown(false)
    e.preventDefault()
  }

  const paint = () => {
    if (!ctx) return
    ctx.fillStyle = 'cornflowerblue'
    ctx.beginPath()
    ctx.moveTo(mousePos().x, mousePos().y)
    ctx.ellipse(
      mousePos().x,
      mousePos().y,
      penSize(),
      penSize(),
      0,
      0,
      2 * Math.PI
    )
    ctx.fill()
    ctx.closePath()
    setDirty(true)
    hypetrigger?.runDebounced(100)
  }

  const clearCanvas = () => {
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    setDirty(false)
    setRecognizedText(undefined)
  }

  const render = () =>
    requestAnimationFrame(() => {
      if (mousedown() && mousePos()) paint()
      render()
    })

  const init = async () => {
    ctx = canvas.getContext('2d')
    await initWasm()
    const scheduler = await initTesseractScheduler({ numWorkers: 1 })
    const trigger = new TesseractTrigger(scheduler)
    hypetrigger = new Hypetrigger(canvas).addTrigger(trigger)
    trigger.onText = (text, timeMS) =>
      setRecognizedText({ text: text.trim(), timeMS: Math.round(timeMS) })
    console.log('Ready.')
    hypetrigger?.runDebounced(100)
  }

  createEffect(() => {
    if (!canvas) return
    console.log('Initializing...')
    init()
    render()
  })

  onCleanup(() => {
    console.log('Cleaning up...')
    hypetrigger?.stop()
  })
  return (
    <>
      <canvas
        ref={canvas}
        id="canvas"
        width="500"
        height="500"
        onMouseMove={e => setMousePos({ x: e.offsetX, y: e.offsetY })}
        onTouchMove={e =>
          setMousePos({
            x: e.touches[0].clientX - canvas.offsetLeft,
            y: e.touches[0].clientY - canvas.offsetTop,
          })
        }
        onMouseDown={startDrawing}
        onMouseUp={stopDrawing}
        onMouseOut={stopDrawing}
        onTouchStart={startDrawing}
        onTouchEnd={stopDrawing}
        onTouchCancel={stopDrawing}
      />
      <div id="right-col">
        <div id="welcome">
          <h1>👈 Say Hello</h1>
          <p>
            <span
              class={!recognizedText()?.text && !dirty() ? 'highlight' : ''}
            >
              Draw words on the canvas to the left
              <br />
              to test how well Hypetrigger can recognize text in realtime.
            </span>
          </p>
        </div>
        <div id="your-wrapper" class={!recognizedText()?.text ? 'hidden' : ''}>
          <span id="your-label">You wrote:</span>
          <div>
            <code id="your-text">{recognizedText()?.text}</code>
            <span
              id="your-ms"
              title="the time it took to recognize the text in your drawing"
            >
              {recognizedText()?.timeMS}ms
            </span>
          </div>
        </div>
        <div id="response-wrapper" class={!responseText() ? 'hidden' : ''}>
          <code id="response-text">
            {responseText()}
            <span class={responseText() ? 'hand wave' : 'hand'}>👋</span>
          </code>
          <span id="response-label">&mdash; Hypetrigger</span>
        </div>
        {dirty() && (
          <div id="clear-btn-container">
            <button onClick={clearCanvas} id="clear-btn">
              Clear Canvas
            </button>
          </div>
        )}
      </div>
    </>
  )
}
