import { PhotonImage } from 'hypetrigger-wasm'
import { Rgb, threshold_color_distance } from '../../../src-wasm/pkg/hypetrigger'

export type ThresholdParams = {
  r: number
  g: number
  b: number
  threshold: number
}

export default function threshold(image: PhotonImage, r: number, g: number, b: number, threshold: number) {
  return threshold_color_distance(image, new Rgb(r, g, b), threshold)
}
