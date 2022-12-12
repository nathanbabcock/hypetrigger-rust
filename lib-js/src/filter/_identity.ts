import { FilterFunction } from '.'

/** The identity filter does nothing; it returns the same image it's given */
export type Identity = {
  type: 'identity'
}

const identity: FilterFunction<Identity> = image => image
export default identity
