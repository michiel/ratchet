// This is a deliberately broken JavaScript file
function(input) {
  // Syntax error - missing closing brace
  const { num1, num2 } = input
  
  if (typeof num1 !== 'number' || typeof num2 !== 'number') {
    throw new Error('num1 and num2 must be numbers');
  
  return {
    sum: num1 + num2
  };
}