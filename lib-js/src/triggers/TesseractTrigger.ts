import { preprocess_image_for_tesseract, Rgb } from '../../../src-wasm/pkg/hypetrigger'
import { tesseractV2 } from '../ai/tesseract'
import crop, { Crop } from '../crop'
import { ThresholdParams } from '../filter/threshold'

export type TesseractTrigger = {
  crop: Crop,
  filter: ThresholdParams,
  regex: string | RegExp,
}

const getRgb = (filter: ThresholdParams) => new Rgb(filter.r, filter.g, filter.b)

// Imperative style
export default function runTesseractTrigger(image: CanvasImageSource, config: TesseractTrigger): string {
  const cropped = crop(image, config.crop)
  const filtered = preprocess_image_for_tesseract(cropped, getRgb(config.filter), config.filter.threshold)
  const recognized = tesseractV2(filtered).data.text
  // const parsed = regex(recognized, config.regex)
  // return parsed
  return recognized
}
