// Exercise the same button whether it is direct or inside a native overflow popover.
export async function showAction(scope,name,index=0){
 const button=scope.getByRole('button',{name,exact:true,includeHidden:true}).nth(index);
 await button.waitFor({state:'attached'});
 if(!await button.isVisible()){
  const popup=button.locator('xpath=ancestor::*[@popover][1]');
  const id=await popup.getAttribute('id');
  await scope.locator(`button[popovertarget=${JSON.stringify(id)}]`).click();
 }
 await button.waitFor();return button;
}
export async function clickAction(scope,name,index=0){await(await showAction(scope,name,index)).click();}
