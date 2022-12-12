export default function regex(result: string, regex: string | RegExp) {
  return !!result.match(new RegExp(regex, 'i'))
}
