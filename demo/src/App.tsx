import { createEffect, createMemo, createSignal, onCleanup } from 'solid-js'
import { Hypetrigger, initWasm } from '../../lib-js/src'
import {
  initTesseractScheduler,
  TesseractTrigger,
} from './../../lib-js/src/tesseract'

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
  let hypetrigger: Hypetrigger | undefined
  const ctx = () => canvas?.getContext('2d', { willReadFrequently: true })

  const [mousedown, setMousedown] = createSignal(false)
  const [mousePos, setMousePos] = createSignal<
    { x: number; y: number } | undefined
  >()
  const [penSize, _setPenSize] = createSignal(5)

  const [yourText, setYourText] = createSignal<string | undefined>()
  const responseText = () =>
    greetings.some(
      greeting =>
        yourText()?.toLowerCase().includes(greeting.toLowerCase()) ?? false
    )
      ? 'Hello to you too!'
      : undefined

  createEffect(() =>
    console.log({ yourText: yourText(), responseText: responseText() })
  )

  const startDrawing = (e: Event) => {
    setMousedown(true)
    e.preventDefault()
  }

  const stopDrawing = (e: Event) => {
    setMousedown(false)
    e.preventDefault()
  }

  const render = () =>
    requestAnimationFrame(() => {
      if (mousedown() && mousePos()) {
        ctx().fillStyle = 'cornflowerblue'
        ctx().moveTo(mousePos().x, mousePos().y)
        ctx().ellipse(
          mousePos().x,
          mousePos().y,
          penSize(),
          penSize(),
          0,
          0,
          2 * Math.PI
        )
        ctx().fill()
      }
      render()
    })

  const init = async () => {
    await initWasm()
    const scheduler = await initTesseractScheduler({ numWorkers: 1 })
    const trigger = new TesseractTrigger(scheduler)
    hypetrigger = new Hypetrigger(canvas).addTrigger(trigger).runRealtime()
    trigger.onText = text => setYourText(text)
  }

  createEffect(() => {
    if (!canvas) return
    init()
    render()
  })

  onCleanup(() => {
    console.log('Cleaning up...')
    if (hypetrigger) hypetrigger.isRunningRealtime = false
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
      ></canvas>
      <div id="right-col">
        <div id="welcome">
          <h1>👈 Say Hello to Hypetrigger.</h1>
          <p>
            Draw words on the canvas to the left to test how well Hypetrigger
            can recognize text in realtime.
          </p>
        </div>
        <div id="your-wrapper" class={!yourText() ? 'hidden' : ''}>
          <span id="your-label">You wrote:</span>
          <code id="your-text">{yourText()}</code>
        </div>
        <div id="response-wrapper" class={!responseText() ? 'hidden' : ''}>
          <code id="response-text">{responseText()}</code>
          <span id="response-label">- Hypetrigger</span>
        </div>
      </div>
    </>
  )
}
