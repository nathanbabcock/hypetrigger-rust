import { AI } from './ai'
import { Crop } from './crop'

/**
 * Describes a function that will map an image (or a single video frame) to a an
 * output such as string (recognized text), number (parsed from the text), or
 * boolean (applying some condition to recognized number or text).
 *
 * Processing occurs in 4 stages:
 * - **Crop** - pick a smaller rectangle out of the full image
 * - **Filter** - pre-process the image to make it easier to recognize
 * - **Recognize** - apply AI, either Tesseract OCR or Tensorflow image classification
 * - **Parse** - post-process the results from the AI to further refine them
 * - **Emit** - do something with the results to trigger event handlers
 */
export type Trigger = {
  /** Unique ID of the trigger */
  id: string

  /** Human-readable name for display in a GUI or elsewhere */
  title: string

  /** Whether to generate debug output for this trigger **/
  debug: boolean

  /** Whether this instance is active and listening for trigger events */
  enabled: boolean

  /** How many seconds to capture in a clip BEFORE the trigger */
  secondsBefore: number

  /** How many seconds to capture in a clip AFTER the trigger */
  secondsAfter: number

  /** Number of consecutive frames which must match before triggering a clip */
  matchesRequired: number | undefined

  /** The id of another trigger to inherit CROP, FILTER, and AI settings from */
  linkedTo: string | undefined

  /** Crop the image down to a smaller rectangle */
  cropFunction: Crop

  // /** An array of filters to apply to the cropped section */
  // filters: Filter[]

  /** An AI method for analyzing the image (e.g. OCR or Image Averaging) */
  ai: AI

  // /** Parser to apply to recognized results */
  // parser: Parser
}
