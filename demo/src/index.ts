import { Hypetrigger, initWasm } from '../../lib-js/src'
import { initTesseractScheduler, TesseractTrigger } from './../../lib-js/src/tesseract'

initWasm()

const canvas = document.getElementById('canvas') as HTMLCanvasElement
const ctx = canvas.getContext('2d', { willReadFrequently: true })!
ctx.fillStyle = 'cornflowerblue'

const state = {
  mousedown: false,
  mouseX: undefined as number | undefined,
  mouseY: undefined as number | undefined,
  penSize: 5,
}

canvas.addEventListener('mousemove', e => {
  state.mouseX = e.offsetX
  state.mouseY = e.offsetY
})
canvas.addEventListener('touchmove', e => {
  state.mouseX = e.touches[0].clientX - canvas.offsetLeft
  state.mouseY = e.touches[0].clientY - canvas.offsetTop
})
const startDrawing = (e: Event) => {
  state.mousedown = true
  e.preventDefault()
}
const stopDrawing = (e: Event) => {
  state.mousedown = false
  e.preventDefault()
}
canvas.addEventListener('mousedown', startDrawing)
canvas.addEventListener('mouseup', stopDrawing)
canvas.addEventListener('mouseout', stopDrawing)
canvas.addEventListener('touchstart', startDrawing)
canvas.addEventListener('touchend', stopDrawing)
canvas.addEventListener('touchcancel', stopDrawing)
const render = () => requestAnimationFrame(() => {
  if (state.mousedown && state.mouseX !== undefined && state.mouseY !== undefined) {
    ctx.moveTo(state.mouseX, state.mouseY)
    ctx.ellipse(state.mouseX, state.mouseY, state.penSize, state.penSize, 0, 0, 2 * Math.PI)
    ctx.fill()
  }
  render()
})
render()

const scheduler = await initTesseractScheduler()
const trigger = new TesseractTrigger(scheduler)
new Hypetrigger(canvas)
  .addTrigger(trigger)
  .runRealtime()
const recognizedText = document.getElementById('recognizedText') as HTMLDivElement
trigger.onText = text => recognizedText.innerText = text
