import { PhotonImage } from '.'
export default abstract class Trigger {
  abstract run(image: PhotonImage): Promise<any>
}
