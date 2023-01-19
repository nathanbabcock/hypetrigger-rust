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

  /** Grab the current frame from the input source and convert it to a PhotonImage */
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

  /** Run all triggers on the given input source (once). */
  async run() {
    let promises = []
    for (const trigger of this.triggers)
      promises.push(trigger.run(this.getPhotonImage()))
    return Promise.all(this.triggers)
  }

  /** Run one trigger at a time (default is in parallel) */
  async runSequentially() {
    for (const trigger of this.triggers)
      await trigger.run(this.getPhotonImage())
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
   * Despite the name, it does not use `setInterval`; instead calls `setTimeout`
   * after each run. This works more reliably than `requestAnimationFrame`, which
   * can overload the browser's memory if it runs fast enough.
   */
  runOnInterval(intervalMS = 100) {
    this.isRunningOnInterval = true
    const callback = async () => {
      if (!this.isRunningOnInterval) return
      await this.run()
      setTimeout(callback, intervalMS)
    }
    callback()
    return this
  }

  /**
   * Re-runs triggers whenever the input changes.
   * 
   * Runs once immediately when the method is called, and then again as needed:
   * - For **image sources**, in a `image.onload` listener
   * - For **video sources**, in a `requestVideoFrameCallback()`
   * - For **canvas sources**, you should use `runDebounced()` or
   *   `runOnInterval()` instead, since there are no events to subscribe to.
   */
  autoRun() {
    if (this.imageSource instanceof HTMLImageElement) {
      if (this.imageSource.complete) this.run()
      this.imageSource.addEventListener('onload', this.run.bind(this))
    } else if (this.imageSource instanceof HTMLVideoElement) {
      if (this.imageSource.readyState === 4) this.run()
      const callback = async () => {
        if (!(this.imageSource instanceof HTMLVideoElement)) return
        await this.run()
        this.imageSource.requestVideoFrameCallback(callback)
      }
      this.imageSource.requestVideoFrameCallback(callback)
    } else if (this.imageSource instanceof HTMLCanvasElement) {
      this.run()
      console.warn('autoRun() only runs once for canvas sources. You should use run(), runDebounced(), or runOnInterval() instead.')
    } else {
      throw new Error('Unsupported image source type')
    }
    this.run()
    return this
  }

  /** Stop running realtime, if applicable */
  stop() {
    this.isRunningOnInterval = false
    return this
  }
}
