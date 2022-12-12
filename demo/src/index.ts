import { initWasm, openImage, preprocessForTesseract, putImageData, Rgb } from 'hypetrigger'

console.log('Hello from TypeScript!')

await initWasm()

const img = document.getElementById('img') as HTMLImageElement
const canvas = document.getElementById('canvas') as HTMLCanvasElement
canvas.width = img.width
canvas.height = img.height
const ctx = canvas.getContext('2d') as CanvasRenderingContext2D
ctx.drawImage(img, 0, 0)

console.time()
const photonImage = openImage(canvas, ctx)
// threshold(photonImage, 100)
const newImage = preprocessForTesseract(photonImage, new Rgb(255, 255, 255), 42)
// const newImage = threshold_color_distance(photonImage, new Rgb(255, 255, 255), 30)
canvas.width = newImage.get_width()
canvas.height = newImage.get_height()
canvas.style.border = '2px dashed black'
console.log(newImage.get_width(), newImage.get_height())
putImageData(canvas, ctx, newImage)
console.timeEnd()
