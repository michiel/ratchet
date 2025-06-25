function sqrt(input) {
  const { number } = input;
  
  if (typeof number !== 'number') {
    throw new Error('Input must be a number');
  }
  
  if (number < 0) {
    throw new Error('Cannot calculate square root of negative number');
  }
  
  const result = Math.sqrt(number);
  
  return { result };
}