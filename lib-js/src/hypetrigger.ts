import { open_image, PhotonImage } from '.'
import { Trigger } from './trigger'

export class Hypetrigger {
  public triggers: Trigger[] = []
  public imageSource: HTMLImageElement | HTMLVideoElement | HTMLCanvasElement
  public isRunningOnInterval: boolean = false
  timeout: number = 0

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

  /** Run all triggers on the given input source (once) */
  run() {
    for (const trigger of this.triggers)
      trigger.run(this.getPhotonImage())
    return this
  }

  /** Run all triggers after `timeoutMS`. Calling this method again resets the timer. */
  runDebounced(timeoutMS = 100) {
    clearTimeout(this.timeout)
    this.timeout = setTimeout(this.run.bind(this), timeoutMS)
    return this
  }

  /**
   * Continuously run all triggers on the given input source.
   * 
   * Despite the name, it does not use `setInterval`; instead it recursively
   * calls `setTimeout` with a small delay. This works more reliably than
   * `requestAnimationFrame`, which can overload the browser's memory if it runs fast enough.
   */
  runOnInterval(intervalMS = 100) {
    this.isRunningOnInterval = true
    const callback = () => {
      if (!this.isRunningOnInterval) return
      this.run()
      setTimeout(callback, intervalMS)
    }
    callback()
    return this
  }

  /** Stop running realtime, if applicable */
  stop() {
    this.isRunningOnInterval = false
    return this
  }
}
