import { openImage, PhotonImage } from '.'

export type Crop = {
  yAnchor: CropYAnchor
  xAnchor: CropXAnchor
  x: number
  y: number
  xUnit: CropUnit
  yUnit: CropUnit
  width: number
  height: number
  widthUnit: CropUnit
  heightUnit: CropUnit
}

export type CropUnit = 'px' | '%'
export type CropYAnchor = 'top' | 'bottom'
export type CropXAnchor = 'left' | 'right'
export type CropRegion = { y: number, x: number, width: number, height: number }

/** Retrieve a rectangle from a Canvas, Image, or Video and return it as a PhotonImage */
export function crop(image: CanvasImageSource, config: Crop, canvas?: HTMLCanvasElement | OffscreenCanvas): PhotonImage {
  if (!canvas) canvas = new OffscreenCanvas(0, 0)
  const region = getCropRegion(config, image)
  const ctx = canvas!.getContext('2d') as CanvasRenderingContext2D
  canvas.width = region.width
  canvas.height = region.height
  ctx.drawImage(image, region.x, region.y, region.width, region.height, 0, 0, region.width, region.height)
  const photonImage = openImage(canvas as HTMLCanvasElement, ctx) // persuade Photon to accept OffscreenCanvas
  return photonImage
}

/**
 * Normalize different formats of cropping (such as % width, or distance from
 * bottom) into a standardized form (px from top/left)
 */
export function getCropRegion(crop: Crop, image: CanvasImageSource | { width: number, height: number }): CropRegion {
  const region: CropRegion = {
    x: crop.x,
    y: crop.y,
    width: crop.width,
    height: crop.height,
  }

  let width: number
  let height: number

  if (image instanceof HTMLVideoElement) {
    width = image.videoWidth
    height = image.videoHeight
  } else if (image instanceof HTMLImageElement) {
    width = image.naturalWidth
    height = image.naturalHeight
  } else if (image instanceof SVGImageElement) {
    width = image.width.baseVal.value
    height = image.height.baseVal.value
  } else {
    width = image.width as number
    height = image.height as number
  }

  if (crop.yUnit === '%') region.y *= height / 100
  if (crop.yAnchor === 'bottom') region.y = height - region.y
  if (crop.heightUnit === '%') region.height *= height / 100

  if (crop.xUnit === '%') region.x *= width / 100
  if (crop.xAnchor === 'right') region.x = width - region.x
  if (crop.widthUnit === '%') region.width *= width / 100

  return region
}
