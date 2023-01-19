import { PhotonImage } from '.'
export abstract class Trigger {
  abstract run(image: PhotonImage): Promise<any>
}

export type TriggerCallback = (image: PhotonImage) => Promise<any>

/** A trivial pass-through trigger that just wraps a callback function */
export class SimpleTrigger extends Trigger {
  callback: TriggerCallback

  constructor(callback: TriggerCallback) {
    super()
    this.callback = callback
  }

  run(image: PhotonImage) {
    return this.callback(image)
  }
}
