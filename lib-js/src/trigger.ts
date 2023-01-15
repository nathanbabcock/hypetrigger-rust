import { PhotonImage } from '.'
export abstract class Trigger {
  abstract run(image: PhotonImage): Promise<any>
}
