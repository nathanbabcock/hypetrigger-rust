import { AIFunction } from '.'

export type Tensorflow = {
  type: 'tensorflow'
}

const tensorflow: AIFunction<Tensorflow> = (image, ai) => {
  throw new Error('not implemented')
}
export default tensorflow
