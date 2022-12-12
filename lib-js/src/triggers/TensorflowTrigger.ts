import { preprocess_image_for_tensorflow } from '../../../src-wasm/pkg/hypetrigger'
import tensorflow from '../ai/tensorflow'
import crop, { Crop } from '../crop'

export type TensorflowTrigger = {
  crop: Crop
}

export default function runTensorflowTrigger(image: CanvasImageSource, config: TensorflowTrigger): string {
  const cropped = crop(image, config.crop)
  const filtered = preprocess_image_for_tensorflow(cropped)
  const recognized = tensorflow(filtered as unknown as any, undefined)
  return recognized
}
