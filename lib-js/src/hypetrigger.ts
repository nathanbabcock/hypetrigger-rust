import { PhotonImage } from '.'
import Trigger from './trigger'

export default class Hypetrigger {
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
    return new PhotonImage(this.imageSource)
  }

  run() {
    for (const trigger of this.triggers)
      trigger.run(this.getPhotonImage())
  }

  runRealtime() {
    this.isRunningRealtime = true
    const callback = () => {
      if (!this.isRunningRealtime) return
      this.run()
      requestAnimationFrame(callback)
    }
    callback()
  }
}
