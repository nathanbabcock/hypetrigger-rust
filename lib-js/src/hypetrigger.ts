import { open_image, PhotonImage } from '.'
import { Trigger } from './trigger'

export class Hypetrigger {
  public triggers: Trigger[] = []
  public imageSource: HTMLImageElement | HTMLVideoElement | HTMLCanvasElement
  public isRunningRealtime: boolean = false

  constructor(imageSource: HTMLImageElement | HTMLVideoElement | HTMLCanvasElement) {
    return this.setImageSource(imageSource)
  }

  setImageSource(imageSource: HTMLImageElement | HTMLVideoElement | HTMLCanvasElement) {
    this.imageSource = imageSource
    return this
  }

  addTrigger(trigger: Trigger) {
    this.triggers.push(trigger)
    return this
  }

  getPhotonImage() {
    let photonImage: PhotonImage
    if (this.imageSource instanceof HTMLCanvasElement) {
      let canvas = this.imageSource
      let ctx = this.imageSource.getContext('2d')
      photonImage = open_image(canvas, ctx)
      return photonImage
    } else {
      throw new Error('Unsupported image source type')
    }
  }

  run() {
    for (const trigger of this.triggers)
      trigger.run(this.getPhotonImage())
    return this
  }

  runRealtime() {
    this.isRunningRealtime = true
    const callback = () => {
      if (!this.isRunningRealtime) return
      this.run()
      requestAnimationFrame(callback)
    }
    requestAnimationFrame(callback)
    return this
  }
}
